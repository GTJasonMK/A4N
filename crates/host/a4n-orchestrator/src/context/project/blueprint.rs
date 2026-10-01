use a4n_api_contract::blueprint::{
    RestoreBlueprintVersionInput, SaveBlueprintInput, UpdateWorldSettingInput,
};
use a4n_db::DbHandle;
use a4n_db_models::models::project::{blueprint_character, blueprint_relationship};
use a4n_domain::project::BlueprintRecord;
use a4n_error::AppError;
use a4n_event::InternalEvent;
use serde_json::Value;

use super::super::OrchestratorContext;
use crate::project::blueprint_restore_error_to_app;
use a4n_domain::collaboration::ProjectAccessRole;

/// Blueprint generation/refinement persistence owner. Public read projections
/// are served by the Blueprint worker; generation keeps raw source reads here.
#[derive(Clone)]
pub(in crate::context) struct ProjectBlueprintApplication {
    database: DbHandle,
}

impl ProjectBlueprintApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &a4n_db::DbHandle {
        &self.database
    }

    async fn current(&self, project_id: i64) -> Result<Option<BlueprintRecord>, AppError> {
        a4n_db::project::novel_blueprint_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::database)
    }

    async fn restore(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        a4n_db::project::restore_blueprint_version_record(self.connection(), project_id, version)
            .await
            .map_err(blueprint_restore_error_to_app)
    }

    async fn blueprint_characters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::project::blueprint_characters_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::database)
    }

    async fn blueprint_relationships_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        a4n_db::project::blueprint_relationships_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::database)
    }
}

impl OrchestratorContext {
    pub(crate) async fn novel_blueprint_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<BlueprintRecord>, AppError> {
        self.project_blueprint.current(project_id).await
    }

    pub(crate) async fn restore_blueprint_version_record(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        let result = self.project_blueprint.restore(project_id, version).await?;
        if let Some(status_change) = &result.status_change {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id,
                from: status_change.from,
                to: status_change.to,
                timestamp: status_change.changed_at,
            })?;
        }
        self.publish_internal_event(InternalEvent::BlueprintUpdated {
            project_id,
            version: result.blueprint.current_version.clone(),
        })?;
        Ok(result)
    }

    pub(crate) async fn restore_blueprint_version_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.restore_blueprint_version_record(project_id, version)
            .await
    }

    pub(crate) async fn update_blueprint_world_setting_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        world_setting: Value,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let result = a4n_db::blueprint::update_blueprint_world_setting_records_transaction(
            self.project_blueprint.connection(),
            a4n_db::blueprint::UpdateBlueprintWorldSettingInput {
                project_id,
                world_setting,
                created_by: format!("user:{user_id}"),
            },
        )
        .await
        .map_err(crate::blueprint::blueprint_db_error)?;
        self.publish_versioned_blueprint_events(&result)?;
        Ok(result)
    }

    pub(crate) async fn save_blueprint_for_user_validated(
        &self,
        user_id: i64,
        input: SaveBlueprintInput,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let result = a4n_db::blueprint::save_manual_blueprint_records_transaction(
            self.project_blueprint.connection(),
            a4n_db::blueprint::SaveManualBlueprintInput {
                project_id: input.project_id,
                genre: input.genre,
                style: input.style,
                tone: input.tone,
                target_audience: input.target_audience,
                one_sentence_summary: input.one_sentence_summary,
                full_synopsis: input.full_synopsis,
                total_chapters: input.total_chapters,
                created_by: format!("user:{user_id}"),
            },
        )
        .await
        .map_err(crate::blueprint::blueprint_db_error)?;
        self.publish_versioned_blueprint_events(&result)?;
        Ok(result)
    }

    pub(crate) async fn load_blueprint(
        &self,
        project_id: i64,
    ) -> Result<Option<BlueprintRecord>, AppError> {
        self.project_blueprint.current(project_id).await
    }

    pub(crate) async fn load_blueprint_characters(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        self.project_blueprint
            .blueprint_characters_for_project(project_id)
            .await
    }

    pub(crate) async fn load_blueprint_relationships(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        self.project_blueprint
            .blueprint_relationships_for_project(project_id)
            .await
    }

    fn publish_versioned_blueprint_events(
        &self,
        result: &a4n_db::blueprint::VersionedBlueprintMutationResult,
    ) -> Result<(), AppError> {
        if let Some(status_change) = result.status_change {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id: result.blueprint.project_id,
                from: status_change.from,
                to: status_change.to,
                timestamp: status_change.changed_at,
            })?;
        }
        self.publish_internal_event(InternalEvent::BlueprintUpdated {
            project_id: result.blueprint.project_id,
            version: result.blueprint.current_version.clone(),
        })?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::blueprint::BlueprintMutationApplicationPort for OrchestratorContext {
    async fn save(&self, user_id: i64, input: SaveBlueprintInput) -> Result<(), AppError> {
        self.save_blueprint_for_user_validated(user_id, input)
            .await?;
        Ok(())
    }

    async fn update_world_setting(
        &self,
        user_id: i64,
        input: UpdateWorldSettingInput,
    ) -> Result<(), AppError> {
        self.update_blueprint_world_setting_for_user_validated(
            user_id,
            input.project_id,
            input.world_setting,
        )
        .await?;
        Ok(())
    }

    async fn restore(
        &self,
        user_id: i64,
        input: RestoreBlueprintVersionInput,
    ) -> Result<(), AppError> {
        self.restore_blueprint_version_for_user_validated(
            user_id,
            input.project_id,
            &input.version,
        )
        .await?;
        Ok(())
    }
}
