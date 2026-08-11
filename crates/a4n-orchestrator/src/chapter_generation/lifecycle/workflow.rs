use std::collections::HashMap;

use serde_json::Value;

use super::super::ChapterGenerationWordCountRange;
use crate::{
    OrchestratorContext,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING},
};

pub(crate) struct StartChapterGenerationWorkflowInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: &'a str,
    pub chapter_title: &'a str,
    pub outline_text: &'a str,
    pub context_before: Option<&'a str>,
    pub word_count_range: ChapterGenerationWordCountRange,
    pub use_rag: bool,
    pub variables: &'a HashMap<String, Value>,
}

impl OrchestratorContext {
    pub(crate) async fn start_chapter_generation_workflow(
        &self,
        input: StartChapterGenerationWorkflowInput<'_>,
    ) -> Result<i64, String> {
        let fingerprint = chapter_generation_workflow_fingerprint(&input);
        let call_key = format!("chapter_generation:chapter:{}", input.chapter_number);
        self.upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "chapter_generation:{}:{}",
                input.project_id, input.chapter_number
            ),
            workflow_kind: "chapter_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("stream".to_string()),
            input_fingerprint_json: Some(fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key),
            active_task_id: Some(input.task_id.to_string()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map(|workflow_run| workflow_run.id)
        .map_err(|error| error.to_string())
    }

    pub(crate) async fn complete_chapter_generation_workflow(
        &self,
        workflow_run_id: i64,
    ) -> Result<(), String> {
        self.mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

pub(super) fn chapter_generation_workflow_fingerprint(
    input: &StartChapterGenerationWorkflowInput<'_>,
) -> Value {
    serde_json::json!({
        "workflow": "chapter_generation",
        "project_id": input.project_id,
        "chapter_number": input.chapter_number,
        "chapter_title": input.chapter_title,
        "outline_text": input.outline_text,
        "context_before": input.context_before,
        "word_count_range": {
            "min": input.word_count_range.min,
            "max": input.word_count_range.max,
        },
        "use_rag": input.use_rag,
        "variables": input.variables,
    })
}
