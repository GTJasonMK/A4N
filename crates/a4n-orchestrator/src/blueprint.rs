use std::collections::HashMap;

use a4n_core::blueprint::{BLUEPRINT_GENERATION_KIND, blueprint_stage_call_key};
use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;
use generation::preview_generate_request;

mod generation;
mod json;
pub mod persistence;
mod refine;

pub(crate) use generation::BLUEPRINT_GENERATION_STAGES;
pub(crate) use json::validate_sparse_blueprint_sections;

impl OrchestratorContext {
    pub async fn preview_blueprint_generation_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<BlueprintGenerationPreviewStep, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.preview_blueprint_generation_request(
            project_id,
            novel_title,
            genre,
            target_length,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn preview_blueprint_generation_request(
        &self,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<BlueprintGenerationPreviewStep, AppError> {
        preview_generate_request(
            project_id,
            novel_title,
            genre,
            target_length,
            prompt_renderer,
            variables,
        )
        .await
    }
}

pub fn blueprint_db_error(error: a4n_db::blueprint::BlueprintDbError) -> AppError {
    match error {
        a4n_db::blueprint::BlueprintDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::blueprint::BlueprintDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::blueprint::BlueprintDbError::Database(error) => AppError::Database(error),
    }
}

pub fn blueprint_sync_error(error: a4n_db::project::BlueprintSyncError) -> AppError {
    match error {
        a4n_db::project::BlueprintSyncError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::BlueprintSyncError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn character_profile_seed_error(
    error: a4n_db::character::CharacterProfileSeedError,
) -> AppError {
    match error {
        a4n_db::character::CharacterProfileSeedError::Database(error) => AppError::Database(error),
        a4n_db::character::CharacterProfileSeedError::Internal(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
    }
}

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_core::models::generation::workflow_run;
use serde_json::json;

impl OrchestratorContext {
    /// 蓝图生成：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_blueprint_generation_workflow_validated(
        &self,
        project_id: i64,
        title: &str,
        genre: &str,
        target_length: i32,
        variables: &serde_json::Value,
        task_id: Option<String>,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        let fingerprint = json!({
            "schema_version": 1,
            "workflow": BLUEPRINT_GENERATION_KIND,
            "project_id": project_id,
            "title": title,
            "genre": genre,
            "target_length": target_length,
            "variables": variables,
        });
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key: format!("{BLUEPRINT_GENERATION_KIND}:project:{project_id}"),
            workflow_kind: BLUEPRINT_GENERATION_KIND.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some("generate".to_string()),
            input_fingerprint_json: Some(fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(blueprint_stage_call_key(BLUEPRINT_GENERATION_STAGES[0].key)),
            active_task_id: task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}
