use a4n_db_models::models::chapter::chapter;
use serde_json::json;
use uuid::Uuid;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

mod checkpoint;

pub use a4n_chapter_content_core::{
    CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
};

pub use a4n_chapter_content_core::{
    chapter_analysis_phase_fingerprint, chapter_analysis_run_input_matches,
    chapter_analysis_run_uses_replacement_commit, chapter_analysis_workflow_fingerprint,
};
pub use checkpoint::{
    ChapterAnalysisResumeBlockReason, chapter_analysis_resume_block_reason,
    require_completed_chapter_analysis_phase,
};

pub fn chapter_analysis_workflow_run_input(
    chapter: &chapter::Model,
    current_step_key: &str,
    active_task_id: Option<&str>,
) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: chapter_analysis_workflow_business_key(chapter.project_id, chapter.id),
        workflow_kind: "chapter_analysis".to_string(),
        project_id: Some(chapter.project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter.id),
        chapter_id: Some(chapter.id),
        mode: None,
        input_fingerprint_json: Some(json!({
            "schema_version": 2,
            "workflow_kind": "chapter_analysis",
            "analysis_revision": Uuid::new_v4(),
            "source_fingerprint": chapter_analysis_workflow_fingerprint(chapter),
        })),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key.to_string()),
        active_task_id: active_task_id.map(str::to_string),
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    }
}

pub fn chapter_analysis_workflow_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("chapter_analysis:{project_id}:{chapter_id}")
}
