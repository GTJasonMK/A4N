use a4n_db_models::models::{chapter::chapter, generation::workflow_run};
use serde_json::{Value, json};

use super::{chapter_analysis_run_input_matches, chapter_analysis_workflow_fingerprint};
use crate::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterAnalysisCheckpointIncomplete {
    step_key: String,
}

impl std::fmt::Display for ChapterAnalysisCheckpointIncomplete {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} 已完成 checkpoint 缺少 completed=true",
            self.step_key
        )
    }
}

impl std::error::Error for ChapterAnalysisCheckpointIncomplete {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChapterAnalysisResumeBlockReason {
    Completed,
    Running,
    NotResumable(String),
}

impl std::fmt::Display for ChapterAnalysisResumeBlockReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => {
                formatter.write_str("当前章节结构化分析派生数据已同步完成，无需继续同步")
            }
            Self::Running => {
                formatter.write_str("当前章节结构化分析正在同步中，请等待完成后再操作")
            }
            Self::NotResumable(reason) => formatter.write_str(reason),
        }
    }
}

pub fn require_completed_chapter_analysis_phase(
    checkpoint_json: &Value,
    step_key: &str,
) -> Result<(), ChapterAnalysisCheckpointIncomplete> {
    if checkpoint_json
        .get("completed")
        .and_then(Value::as_bool)
        .is_some_and(|completed| completed)
    {
        Ok(())
    } else {
        Err(ChapterAnalysisCheckpointIncomplete {
            step_key: step_key.to_string(),
        })
    }
}

pub fn chapter_analysis_phase_fingerprint(chapter: &chapter::Model, step_key: &str) -> Value {
    json!({
        "schema_version": 1,
        "workflow": "chapter_analysis.derived_sync",
        "step_key": step_key,
        "chapter": chapter_analysis_workflow_fingerprint(chapter),
        "analysis_data": chapter.analysis_data.as_ref(),
    })
}

pub fn chapter_analysis_resume_block_reason(
    run: &workflow_run::Model,
    chapter: &chapter::Model,
) -> Option<ChapterAnalysisResumeBlockReason> {
    if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter) {
        return None;
    }
    if run.status == WORKFLOW_STATUS_COMPLETED && !run.can_resume {
        return Some(ChapterAnalysisResumeBlockReason::Completed);
    }
    if run.status == WORKFLOW_STATUS_RUNNING {
        return Some(ChapterAnalysisResumeBlockReason::Running);
    }
    if let Err(reason) = crate::workflow::require_workflow_can_resume(
        &run.status,
        run.can_resume,
        run.resume_block_reason.as_deref(),
    ) {
        return Some(ChapterAnalysisResumeBlockReason::NotResumable(reason));
    }
    None
}
