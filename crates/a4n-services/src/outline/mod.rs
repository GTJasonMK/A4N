//! 大纲系统服务：分部大纲 + 章节大纲生成/管理
//!
//! 对应文档 04-大纲系统

mod chapter_contract;
mod chapters;
mod context;
mod part_contract;
mod parts;

use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::llm::provider::{LlmProvider, is_truncated_finish_reason};
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::Value;

#[derive(Clone)]
pub struct OutlineService {
    db: Arc<DatabaseConnection>,
}

pub type ProgressReporter<'a> =
    Option<&'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync)>;

pub struct ChapterOutlineGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub part_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterOutlineCountGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterOutlineRegenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub outline_id: i64,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct BeatMappingApplyInput<'a, 'progress> {
    pub project_id: i64,
    pub part_id: i64,
    pub beat_sheet: &'a str,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineRegenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineContinuationInput<'a> {
    pub project_id: i64,
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OutlineRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

impl OutlineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn ready_outline_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<String>,
    request: ChatRequest,
) -> OutlineRequestPreviewStep {
    OutlineRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.into(),
        state: "ready".to_string(),
        note: None,
        request: Some(request),
    }
}

fn reused_outline_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<String>,
    note: impl Into<String>,
) -> OutlineRequestPreviewStep {
    OutlineRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.into(),
        state: "reused".to_string(),
        note: Some(note.into()),
        request: None,
    }
}

fn blocked_outline_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<String>,
    note: impl Into<String>,
) -> OutlineRequestPreviewStep {
    OutlineRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.into(),
        state: "blocked".to_string(),
        note: Some(note.into()),
        request: None,
    }
}

fn report_progress(
    progress: ProgressReporter<'_>,
    current: i32,
    total: i32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress {
        let current = u32::try_from(current.max(0)).unwrap_or(u32::MAX);
        let total = u32::try_from(total.max(1)).unwrap_or(u32::MAX);
        report(current.min(total), total, message)?;
    }
    Ok(())
}

fn ensure_response_not_truncated(finish_reason: Option<&str>, label: &str) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "模型生成{label}时达到当前场景输出上限，返回 JSON 被截断；请在设置页调大大纲生成 max_tokens，或减少生成范围/输出细节后重试"
        )));
    }
    Ok(())
}

fn validate_existing_chapter_outline_for_request(
    outline: &chapter_outline::Model,
    chapter_number: i32,
    expected_part_id: Option<i64>,
) -> Result<(), AppError> {
    if outline.chapter_number != chapter_number {
        return Err(AppError::InvalidArgument(format!(
            "已有章纲章节号 {} 与当前请求第 {chapter_number} 章不一致",
            outline.chapter_number
        )));
    }
    if outline.part_id != expected_part_id {
        return Err(AppError::InvalidArgument(format!(
            "已有第 {chapter_number} 章章纲关联分部 {:?} 与当前计划 {:?} 不一致，请先删除旧章纲或重新生成对应范围",
            outline.part_id, expected_part_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
