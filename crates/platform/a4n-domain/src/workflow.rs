//! 工作流输入门禁与持久化读取投影。

use a4n_progress::ProgressMeta;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod resume;
mod status;
mod validation;

pub use resume::*;
pub use status::*;
pub use validation::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunSummary {
    pub id: i64,
    pub business_key: String,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub mode: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub status: String,
    pub current_step_key: Option<String>,
    pub progress: Option<WorkflowRunProgressSummary>,
    #[serde(skip_serializing)]
    pub progress_json: Option<Value>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCheckpointSummary {
    pub id: i64,
    pub workflow_run_id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub generation_run_ids_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowRunResumeRecord {
    pub id: i64,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub input_fingerprint_json: Option<Value>,
    pub status: String,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_input_guards_are_explicit() {
        assert!(require_positive_workflow_run_id(1).is_ok());
        assert!(require_positive_workflow_run_id(0).is_err());
        assert!(validate_task_history_limit(1).is_ok());
        assert_eq!(
            validate_task_history_limit(0).unwrap_err(),
            "limit 必须大于 0"
        );
        assert!(validate_optional_project_id(None).is_ok());
        assert!(validate_optional_project_id(Some(1)).is_ok());
        assert!(validate_optional_project_id(Some(0)).is_err());
    }

    #[test]
    fn cancellation_guards_cover_target_and_terminal_statuses() {
        assert!(cancel_workflow_requires_target(true, false).is_ok());
        assert!(cancel_workflow_requires_target(false, true).is_ok());
        assert!(cancel_workflow_requires_target(false, false).is_err());
        assert!(is_video_status_final_for_cancel_interrupt("completed"));
        assert!(is_video_status_final_for_cancel_interrupt("failed"));
        assert!(is_video_status_final_for_cancel_interrupt("cancelled"));
        assert!(is_video_status_final_for_cancel_interrupt("interrupted"));
        assert!(!is_video_status_final_for_cancel_interrupt("running"));
        assert!(!is_video_status_final_for_cancel_interrupt("pending"));
    }

    #[test]
    fn scene_profile_sync_resume_requires_project_scope() {
        assert_eq!(
            resolve_unified_resume_dispatch("scene_profile_sync", Some("novel_project"), Some(7))
                .unwrap(),
            UnifiedResumeDispatch::SceneProfileSync
        );
        assert!(
            resolve_unified_resume_dispatch("scene_profile_sync", Some("chapter"), Some(7))
                .is_err()
        );
    }
}
