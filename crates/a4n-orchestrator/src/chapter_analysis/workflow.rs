use a4n_core::chapter_analysis::validate_saved_chapter_analysis;
use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

mod checkpoint;

pub use checkpoint::{
    ChapterAnalysisCheckpointIncomplete, ChapterAnalysisResumeBlockReason,
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    require_completed_chapter_analysis_phase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousChapterAnalysisIncomplete {
    operation: String,
    target_chapter_number: i32,
    gaps: Vec<String>,
}

impl std::fmt::Display for PreviousChapterAnalysisIncomplete {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let details = self
            .gaps
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("；");
        let omitted = self.gaps.len().saturating_sub(8);
        write!(
            formatter,
            "{}不能跳过前序章节：第 {} 章之前还有 {} 章未完成分析顺序校验：{}{}",
            self.operation,
            self.target_chapter_number,
            self.gaps.len(),
            details,
            if omitted > 0 {
                format!("；另有 {omitted} 章未列出")
            } else {
                String::new()
            }
        )
    }
}

impl std::error::Error for PreviousChapterAnalysisIncomplete {}

pub fn chapter_analysis_workflow_run_input(
    chapter: &chapter::Model,
    current_step_key: &str,
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
        input_fingerprint_json: Some(chapter_analysis_workflow_fingerprint(chapter)),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key.to_string()),
        active_task_id: None,
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
    })
}

pub fn chapter_analysis_run_input_matches(
    run_fingerprint_json: &Option<Value>,
    chapter: &chapter::Model,
) -> bool {
    run_fingerprint_json
        .as_ref()
        .is_some_and(|fingerprint| *fingerprint == chapter_analysis_workflow_fingerprint(chapter))
}

pub fn ensure_previous_chapter_analysis_complete_from_rows(
    operation: &str,
    target_chapter_number: i32,
    previous: &[chapter::Model],
) -> Result<(), PreviousChapterAnalysisIncomplete> {
    let incomplete = previous
        .iter()
        .filter(|chapter| chapter.status != "deleted")
        .filter_map(previous_chapter_analysis_gap)
        .collect::<Vec<_>>();
    if incomplete.is_empty() {
        return Ok(());
    }

    Err(PreviousChapterAnalysisIncomplete {
        operation: operation.to_string(),
        target_chapter_number,
        gaps: incomplete,
    })
}

fn previous_chapter_analysis_gap(chapter: &chapter::Model) -> Option<String> {
    if chapter.status == chapter::STATUS_GENERATING {
        return Some(format!("第{}章正在生成正文", chapter.chapter_number));
    }
    if !chapter.has_available_content() {
        return Some(format!("第{}章正文为空", chapter.chapter_number));
    }
    let Some(analysis) = chapter.analysis_data.as_ref() else {
        return Some(format!("第{}章未生成结构化分析", chapter.chapter_number));
    };
    if let Err(error) = validate_saved_chapter_analysis(analysis) {
        return Some(format!(
            "第{}章结构化分析数据无效：{error}",
            chapter.chapter_number
        ));
    }
    None
}
