use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

mod checkpoint;

pub const CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP: &str = "chapter_analysis.replacement_commit";
pub const CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP: &str = "chapter_analysis.replacement_cleanup";

pub use checkpoint::{
    ChapterAnalysisCheckpointIncomplete, ChapterAnalysisResumeBlockReason,
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
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
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    }
}

pub fn chapter_analysis_workflow_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("chapter_analysis:{project_id}:{chapter_id}")
}

pub fn chapter_analysis_workflow_fingerprint(chapter: &chapter::Model) -> Value {
    json!({
        "schema_version": 1,
        "workflow_kind": "chapter_analysis",
        "project_id": chapter.project_id,
        "chapter_id": chapter.id,
        "chapter_number": chapter.chapter_number,
        "content_sha256": source_content_sha256(&chapter.content),
        "title": chapter.title,
        "updated_at": chapter.updated_at,
    })
}

pub fn chapter_analysis_run_input_matches(
    run_fingerprint_json: &Option<Value>,
    chapter: &chapter::Model,
) -> bool {
    let Some(fingerprint) = run_fingerprint_json.as_ref() else {
        return false;
    };
    let source = if fingerprint.get("schema_version").and_then(Value::as_i64) == Some(2)
        && fingerprint.get("workflow_kind").and_then(Value::as_str) == Some("chapter_analysis")
        && fingerprint
            .get("analysis_revision")
            .and_then(Value::as_str)
            .is_some_and(|revision| !revision.is_empty())
    {
        fingerprint.get("source_fingerprint")
    } else {
        Some(fingerprint)
    };
    source == Some(&chapter_analysis_workflow_fingerprint(chapter))
}

pub fn chapter_analysis_run_uses_replacement_commit(run_fingerprint_json: &Option<Value>) -> bool {
    run_fingerprint_json.as_ref().is_some_and(|fingerprint| {
        fingerprint.get("schema_version").and_then(Value::as_i64) == Some(2)
            && fingerprint.get("workflow_kind").and_then(Value::as_str) == Some("chapter_analysis")
            && fingerprint
                .get("analysis_revision")
                .and_then(Value::as_str)
                .is_some_and(|revision| !revision.is_empty())
            && fingerprint.get("source_fingerprint").is_some()
    })
}
