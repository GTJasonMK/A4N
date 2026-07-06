use super::ChapterService;
use super::response_json::{
    GeneratedChapterMetadata, normalize_generated_chapter_content_layout,
    normalize_generated_chapter_response,
};
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter::{self, STATUS_DRAFT, STATUS_GENERATING};
use a4n_engine::llm::provider::ChatRequest;
use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::OwnedMutexGuard;

use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, chat_request_messages_json,
    context_aware_chat_request_trace_json, delete_generation_run_targets_for,
    record_completed_generation_run, text_response_summary_json,
};

pub(super) struct ChapterGenerationSession {
    project_id: i64,
    chapter_number: i32,
    existing_chapter: Option<ExistingChapterGeneration>,
    _guard: OwnedMutexGuard<()>,
}

struct ExistingChapterGeneration {
    id: i64,
    previous_status: String,
    title: Option<String>,
    summary: Option<String>,
}

pub(super) struct ChapterGenerationTrace {
    pub request: ChatRequest,
    pub finish_reason: Option<String>,
    pub model_name: String,
}

impl ChapterService {
    pub(super) async fn begin_generation(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<ChapterGenerationSession, AppError> {
        let guard = self.try_generation_guard(project_id, chapter_number)?;
        let chapter_model = self.find_chapter(project_id, chapter_number).await?;
        self.begin_generation_for_chapter(project_id, chapter_number, chapter_model, guard)
            .await
    }

    fn try_generation_guard(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<OwnedMutexGuard<()>, AppError> {
        let key = (project_id, chapter_number);
        let lock = self
            .generation_locks
            .entry(key)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        lock.try_lock_owned().map_err(|_| AppError::InvalidState {
            current: STATUS_GENERATING.to_string(),
            operation: format!("生成第{chapter_number}章"),
        })
    }

    async fn begin_generation_for_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        chapter_model: Option<chapter::Model>,
        guard: OwnedMutexGuard<()>,
    ) -> Result<ChapterGenerationSession, AppError> {
        let Some(chapter_model) = chapter_model else {
            return Ok(ChapterGenerationSession {
                project_id,
                chapter_number,
                existing_chapter: None,
                _guard: guard,
            });
        };

        if chapter_model.is_generating() {
            return Err(AppError::InvalidState {
                current: chapter_model.status,
                operation: format!("生成第{chapter_number}章"),
            });
        }

        let previous_status = chapter_model.status.clone();
        let mut active: chapter::ActiveModel = chapter_model.into();
        active.status = Set(STATUS_GENERATING.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let chapter = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(ChapterGenerationSession {
            project_id,
            chapter_number,
            existing_chapter: Some(ExistingChapterGeneration {
                id: chapter.id,
                previous_status,
                title: chapter.title,
                summary: chapter.summary,
            }),
            _guard: guard,
        })
    }

    pub(super) async fn restore_generation_status(&self, session: ChapterGenerationSession) {
        let Some(existing_chapter) = session.existing_chapter else {
            return;
        };

        let current = match chapter::Entity::find_by_id(existing_chapter.id)
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(chapter)) => chapter,
            Ok(None) => return,
            Err(err) => {
                tracing::warn!(
                    chapter_id = existing_chapter.id,
                    error = %err,
                    "章节生成失败后恢复状态失败：查询章节失败"
                );
                return;
            }
        };

        if !current.is_generating() {
            return;
        }

        let mut active: chapter::ActiveModel = current.into();
        active.status = Set(existing_chapter.previous_status);
        active.updated_at = Set(Utc::now().naive_utc());
        if let Err(err) = active.update(self.db.as_ref()).await {
            tracing::warn!(
                chapter_id = existing_chapter.id,
                error = %err,
                "章节生成失败后恢复状态失败：更新章节失败"
            );
        }
    }

    pub(super) async fn finish_generated_chapter(
        &self,
        session: &ChapterGenerationSession,
        content: &str,
        chapter_outline: &str,
        _change_description: &str,
        generation_trace: Option<ChapterGenerationTrace>,
    ) -> Result<chapter::Model, AppError> {
        let validated =
            self.validate_generated_chapter_for_checkpoint(session, content, chapter_outline)?;

        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let word_count = validated.normalized_content.chars().count() as i32;
        let now = Utc::now().naive_utc();
        let updated_chapter = match &session.existing_chapter {
            Some(existing_chapter) => {
                let current = chapter::Entity::find_by_id(existing_chapter.id)
                    .one(&txn)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("章节不存在: {}", existing_chapter.id))
                    })?;
                if !current.is_generating() {
                    return Err(AppError::InvalidState {
                        current: current.status,
                        operation: "完成章节生成".to_string(),
                    });
                }

                let mut active: chapter::ActiveModel = current.into();
                active.content = Set(validated.normalized_content.clone());
                active.word_count = Set(word_count);
                active.title = Set(Some(validated.title.clone()));
                active.summary = Set(Some(validated.summary.clone()));
                active.style_tags = Set(None);
                active.analysis_data = Set(None);
                active.status = Set(STATUS_DRAFT.to_string());
                active.updated_at = Set(now);
                active.update(&txn).await.map_err(AppError::Database)?
            }
            None => chapter::ActiveModel {
                project_id: Set(session.project_id),
                chapter_number: Set(session.chapter_number),
                title: Set(Some(validated.title.clone())),
                content: Set(validated.normalized_content.clone()),
                word_count: Set(word_count),
                status: Set(STATUS_DRAFT.to_string()),
                summary: Set(Some(validated.summary.clone())),
                outline_id: Set(None),
                style_tags: Set(None),
                analysis_data: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await
            .map_err(AppError::Database)?,
        };
        delete_generation_run_targets_for(&txn, "chapter_content", &[updated_chapter.id]).await?;
        delete_generation_run_targets_for(&txn, "chapter_analysis", &[updated_chapter.id]).await?;
        txn.commit().await.map_err(AppError::Database)?;
        if let Some(trace) = generation_trace {
            record_completed_generation_run(
                self.db.as_ref(),
                RecordCompletedGenerationRunInput {
                    project_id: updated_chapter.project_id,
                    workflow: "chapter_generation".to_string(),
                    step_key: format!("chapter_{}", updated_chapter.chapter_number),
                    title: "生成章节正文".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: Some(trace.model_name),
                    prompt_template_names_json: Some(serde_json::json!(["writing"])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(chat_request_messages_json(&trace.request)?),
                    rendered_prompts_json: None,
                    input_variables_json: Some(serde_json::json!({
                        "chapter_number": updated_chapter.chapter_number,
                        "chapter_outline": chapter_outline,
                    })),
                    source_refs_json: Some(serde_json::json!({
                        "source_node": "chapter_outlines",
                        "source_kind": "chapter_outline",
                    })),
                    media_refs_json: None,
                    request_json: Some(context_aware_chat_request_trace_json(&trace.request)?),
                    response_summary_json: Some(text_response_summary_json(
                        content,
                        trace.finish_reason.as_deref(),
                        serde_json::json!({
                            "title": updated_chapter.title,
                            "word_count": updated_chapter.word_count,
                        }),
                    )),
                    targets: vec![GenerationRunTraceTarget {
                        target_kind: "chapter_content".to_string(),
                        target_table: "chapter_content".to_string(),
                        target_id: updated_chapter.id,
                    }],
                },
            )
            .await?;
        }

        Ok(updated_chapter)
    }

    pub(super) fn validate_generated_chapter_for_checkpoint(
        &self,
        session: &ChapterGenerationSession,
        content: &str,
        chapter_outline: &str,
    ) -> Result<ValidatedGeneratedChapter, AppError> {
        let metadata = generated_chapter_metadata(session, chapter_outline, content);
        let payload = normalize_generated_chapter_response(content, &metadata)?;
        let normalized_content = normalize_generated_chapter_content_layout(&payload.content);
        let normalized_content = normalized_content.trim();
        if normalized_content.is_empty() {
            return Err(AppError::InvalidArgument(
                "LLM 未返回可用章节正文，未写入正文".to_string(),
            ));
        }
        Ok(ValidatedGeneratedChapter {
            title: payload.title,
            summary: payload.summary,
            normalized_content: normalized_content.to_string(),
        })
    }

    async fn find_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter::Model>, AppError> {
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.eq(chapter_number))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}

pub(super) struct ValidatedGeneratedChapter {
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) normalized_content: String,
}

fn generated_chapter_metadata(
    session: &ChapterGenerationSession,
    chapter_outline: &str,
    raw_content: &str,
) -> GeneratedChapterMetadata {
    let outline_title =
        extract_marked_outline_value(chapter_outline, &["章节标题", "标题", "title"]);
    let outline_summary = extract_marked_outline_value(
        chapter_outline,
        &["章节摘要", "摘要", "summary", "大纲摘要"],
    );
    let title = session
        .existing_chapter
        .as_ref()
        .and_then(|chapter| chapter.title.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or(outline_title)
        .unwrap_or_else(|| format!("第{}章", session.chapter_number));
    let summary = session
        .existing_chapter
        .as_ref()
        .and_then(|chapter| chapter.summary.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or(outline_summary)
        .unwrap_or_else(|| summary_from_generated_content(raw_content));
    GeneratedChapterMetadata { title, summary }
}

fn extract_marked_outline_value(outline: &str, labels: &[&str]) -> Option<String> {
    for line in outline.lines() {
        let line = line.trim().trim_start_matches(['#', '-', '*', ' ']).trim();
        for label in labels {
            for separator in ['：', ':'] {
                let prefix = format!("{label}{separator}");
                if let Some(value) = line.strip_prefix(&prefix) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

fn summary_from_generated_content(raw_content: &str) -> String {
    let mut summary = raw_content
        .trim()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("章节正文已生成")
        .chars()
        .take(120)
        .collect::<String>();
    if summary.chars().count() == 120 {
        summary.push('…');
    }
    summary
}
