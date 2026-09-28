//! 无状态工作流输入门禁。

mod resume;
mod status;
mod validation;

pub use resume::*;
pub use status::*;
pub use validation::*;

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
