use crate::OrchestratorContext;
use a4n_core::error::AppError;
pub use a4n_db::scene_profile::{
    SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileRecordDraft,
    SceneProfileRecordInput, SceneSyncRecordInput, SceneSyncRecordResult,
    SceneTransitionRecordInput,
};
mod application;
mod contracts;
mod mapping;
mod rules;
mod workflow_rules;

pub use application::{
    SCENE_PROFILE_SYNC_PROGRESS_TOTAL, SceneProfileSyncHostPort, SceneProfileSyncPreviewStep,
    SceneProfileSyncProgress, SceneProfileSyncProgressReporter, SceneProfileSyncTaskScope,
};
pub use contracts::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SCENE_TRANSITION_MOVEMENT_TYPES, SCENE_TYPES,
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneSyncResult, SceneTransitionDraft,
};
pub use mapping::{
    normalize_scene_profile_record_input, normalize_scene_sync_record_input,
    scene_sync_result_from_record,
};
pub use rules::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_type_value,
};
pub use workflow_rules::{
    scene_profile_sync_business_key, scene_profile_sync_workflow_fingerprint,
};

#[cfg(test)]
mod tests;

pub(crate) fn scene_profile_db_error(
    error: a4n_db::scene_profile::SceneProfileDbError,
) -> AppError {
    match error {
        a4n_db::scene_profile::SceneProfileDbError::Database(error) => AppError::Database(error),
        a4n_db::scene_profile::SceneProfileDbError::NotFound(message) => {
            AppError::NotFound(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
    }
}

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_core::models::chapter::chapter;
use a4n_core::models::generation::workflow_run;

impl OrchestratorContext {
    /// 场景档案同步：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_scene_profile_sync_workflow_validated(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        chapters: &[chapter::Model],
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        let mode = chapter_number
            .map(|value| format!("chapter:{value}"))
            .unwrap_or_else(|| "all".to_string());
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
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
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}
