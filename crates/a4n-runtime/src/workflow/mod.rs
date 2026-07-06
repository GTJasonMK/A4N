use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_services::chapter::post_process::ChapterPostProcessor;
use sea_orm::EntityTrait;

mod chapter_analysis;
mod chapter_cleanup;
mod chapter_context;
mod project_status;
mod rag_records;

pub(crate) use chapter_analysis::chapter_analysis_run_input_matches;
pub use chapter_analysis::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisProgress, GenerateChapterAnalysisInput,
    ensure_previous_chapter_analysis_complete, generate_and_sync_chapter_analysis_with_progress,
    generate_chapter_analysis_with_progress, preflight_generate_chapter_analysis,
    preflight_resume_chapter_analysis_sync, preview_chapter_analysis_request,
    preview_resume_chapter_analysis_sync_request, resume_chapter_analysis_sync_with_progress,
    resume_chapter_analysis_workflow_run, sync_project_state_after_chapter_analysis_with_progress,
};
pub use chapter_cleanup::{cleanup_chapter_derived_data, delete_latest_chapter_outlines_cascade};
pub use chapter_context::resolve_chapter_context_before;
pub use project_status::{
    advance_project_status_if_allowed, character_sync_chapters, refresh_project_current_chapter,
    sync_project_completion_status, sync_project_status_from_artifacts,
    validate_chapter_generation_ready,
};
pub(crate) use rag_records::{chapter_rag_record_label, chapter_rag_records};
pub use rag_records::{
    delete_character_attribute_event_rag_by_ids, delete_character_state_rag_by_ids,
    delete_foreshadowing_rag_by_ids, delete_protagonist_rag_by_ids,
    ingest_character_attribute_event_rag, ingest_character_state_rag, ingest_foreshadowing_rag,
    reingest_foreshadowing_rag_by_ids, sync_chapter_rag_records,
};

pub async fn postprocess_chapter_after_content_change(
    state: &AppState,
    chapter_id: i64,
) -> Result<(), AppError> {
    let before = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;

    cleanup_chapter_derived_data(
        state,
        before.project_id,
        before.id,
        before.chapter_number,
        false,
    )
    .await
    .map_err(|err| {
        AppError::Internal(anyhow::anyhow!(
            "章节后处理停止：清理章节派生数据失败: {err}"
        ))
    })?;

    if !before.has_available_content() {
        refresh_project_current_chapter(state, before.project_id).await?;
        sync_project_completion_status(state, before.project_id).await?;
        return Ok(());
    }

    let processor = ChapterPostProcessor::new(state.db.clone());
    processor.process_chapter(chapter_id).await?;

    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;

    refresh_project_current_chapter(state, ch.project_id).await?;
    sync_project_status_from_artifacts(state, ch.project_id).await?;
    Ok(())
}

pub async fn preflight_chapter_content_change(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    chapter_cleanup::preflight_chapter_derived_cleanup(state, project_id, chapter_number)
        .await
        .map_err(|err| AppError::InvalidState {
            current: "章节派生数据无法清理".to_string(),
            operation: format!("保存章节正文前预检: {err}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::embedding::provider::{EmbeddingBackend, EmbeddingProvider};
    use async_trait::async_trait;

    struct TopicEmbeddingProvider;

    #[async_trait]
    impl EmbeddingProvider for TopicEmbeddingProvider {
        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
            Ok(texts
                .into_iter()
                .map(|text| {
                    if text.contains("星门") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect())
        }

        fn dimension(&self) -> usize {
            2
        }

        fn model_name(&self) -> &str {
            "topic-test"
        }

        fn backend(&self) -> EmbeddingBackend {
            EmbeddingBackend::LocalHash
        }
    }

    fn chapter_for_sync(status: &str, content: &str) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: 1,
            project_id: 1,
            outline_id: None,
            chapter_number: 1,
            title: Some("测试章节".to_string()),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: status.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn chapter_rag_chunks_use_semantic_dp_without_overlap() {
        let chapter = chapter_for_sync(
            "completed",
            "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
        );
        let provider = TopicEmbeddingProvider;

        let chunks = rag_records::chapter_rag_chunks(&chapter, &provider)
            .await
            .expect("chapter chunks");

        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.contains("星门亮起"));
        assert!(!chunks[0].content.contains("厨房起火"));
        assert!(chunks[1].content.contains("厨房起火"));
        assert!(!chunks[1].content.starts_with("[...]"));
        assert_eq!(chunks[0].sentence_count, 3);
        assert!(chunks[0].density_score > 0.0);
    }

    #[tokio::test]
    async fn chapter_rag_records_include_semantic_chunk_metadata() {
        let chapter = chapter_for_sync(
            "completed",
            "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
        );
        let provider = TopicEmbeddingProvider;

        let records = rag_records::chapter_rag_records(&chapter, &provider)
            .await
            .expect("chapter records");
        let paragraph_records = records
            .iter()
            .filter(|record| {
                record
                    .metadata
                    .get("content_kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("paragraph")
            })
            .collect::<Vec<_>>();

        assert_eq!(paragraph_records.len(), 2);
        assert_eq!(paragraph_records[0].metadata["semantic_chunked"], true);
        assert_eq!(paragraph_records[0].metadata["chunk_count"], 2);
        assert_eq!(paragraph_records[0].metadata["sentence_count"], 3);
        assert_eq!(paragraph_records[0].metadata["start_sentence_idx"], 0);
        assert_eq!(paragraph_records[0].metadata["end_sentence_idx"], 3);
    }

    #[tokio::test]
    async fn expected_chapter_rag_record_ids_match_strategy_chunk_count() {
        let mut chapter = chapter_for_sync(
            "completed",
            "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
        );
        chapter.summary = Some("主角完成一次关键判断。".to_string());
        let provider = TopicEmbeddingProvider;

        let chunks = rag_records::chapter_rag_chunks(&chapter, &provider)
            .await
            .expect("chapter chunks");
        let ids = rag_records::expected_chapter_rag_record_ids(&chapter, &provider)
            .await
            .expect("expected ids");

        assert_eq!(chunks.len(), 2);
        assert_eq!(
            ids,
            vec![
                "chapter:1:1:p1".to_string(),
                "chapter:1:1:p2".to_string(),
                "chapter:1:1:summary".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn chapter_rag_records_include_analysis_outputs() {
        let mut chapter = chapter_for_sync("completed", "林岚在旧城发现暗号。");
        chapter.analysis_data = Some(serde_json::json!({
            "metadata": {
                "characters": ["林岚"],
                "locations": ["旧城"],
                "items": [],
                "tags": ["追查"],
                "tone": "紧张",
                "timeline_marker": null
            },
            "key_events": [
                { "description": "林岚发现旧城暗号", "importance": "high" }
            ],
            "foreshadowing": {
                "planted": [
                    {
                        "description": "暗号指向后续叛徒身份",
                        "priority": "high",
                        "category": "plot_twist"
                    }
                ]
            }
        }));
        let provider = TopicEmbeddingProvider;

        let ids = rag_records::expected_chapter_rag_record_ids(&chapter, &provider)
            .await
            .expect("expected ids");

        assert!(ids.contains(&"chapter:1:1:p1".to_string()));
        assert!(ids.contains(&"chapter:1:1:metadata".to_string()));
        assert!(ids.contains(&"chapter:1:1:event1".to_string()));
        assert!(ids.contains(&"chapter:1:1:foreshadow1".to_string()));
    }
}
