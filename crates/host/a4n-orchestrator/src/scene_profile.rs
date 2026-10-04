use crate::OrchestratorContext;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
pub(crate) use a4n_scene_host_contract::{
    SceneProfileSyncApplicationPort, SceneProfileSyncHostPort, SceneProfileSyncProgress,
};
mod application;
mod contracts;
mod mapping;
mod rules;
mod workflow_rules;

pub(crate) use contracts::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SCENE_TRANSITION_MOVEMENT_TYPES, SCENE_TYPES,
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneTransitionDraft,
};
pub(crate) use mapping::normalize_scene_sync_record_input;
pub(crate) use rules::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_type_value,
};
pub(crate) use workflow_rules::{
    scene_profile_sync_business_key, scene_profile_sync_workflow_fingerprint,
};

#[cfg(test)]
mod tests;

pub(crate) fn scene_profile_db_error(
    error: a4n_db::scene_profile::SceneProfileDbError,
) -> AppError {
    match error {
        a4n_db::scene_profile::SceneProfileDbError::Database(error) => AppError::database(error),
        a4n_db::scene_profile::SceneProfileDbError::NotFound(message) => {
            AppError::NotFound(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
    }
}

#[async_trait::async_trait]
impl SceneProfileSyncApplicationPort for OrchestratorContext {
    async fn sync(
        &self,
        user_id: i64,
        input: a4n_api_contract::profile::SyncSceneProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<(), AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.sync_scene_profiles_validated(
            user_id,
            input.project_id,
            input.chapter_number,
            prompt_renderer,
            host_port,
            progress,
            None,
        )
        .await
    }

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        self.verify_project_permission(
            run.project_id.ok_or_else(|| {
                AppError::InvalidArgument("场景档案同步工作流缺少 project_id".to_string())
            })?,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.resume_scene_profile_sync_workflow_run_validated(
            user_id,
            &run,
            prompt_renderer,
            host_port,
        )
        .await
    }
}

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_domain::chapter::ChapterRecord;

impl OrchestratorContext {
    /// 场景档案同步：校验后创建或恢复 workflow_run（RUNNING）。
    pub(crate) async fn begin_scene_profile_sync_workflow_validated(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        chapters: &[ChapterRecord],
        active_task_id: Option<&str>,
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        let mode = chapter_number
            .map(|value| format!("chapter:{value}"))
            .unwrap_or_else(|| "all".to_string());
        self.begin_or_resume_workflow_run_validated(UpsertWorkflowRunInput {
            business_key: scene_profile_sync_business_key(project_id, chapter_number),
            workflow_kind: "scene_profile_sync".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(mode),
            input_fingerprint_json: Some(scene_profile_sync_workflow_fingerprint(
                project_id,
                chapter_number,
                chapters,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("scene_profile_sync.prepare".to_string()),
            active_task_id: active_task_id.map(str::to_string),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}
