use super::{ChapterService, ensure_chapter_response_not_truncated};
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING, WorkflowRunService};
use a4n_core::error::AppError;
use a4n_core::event::bridge::BroadcastEventBus;
use a4n_core::models::chapter::chapter::{self, STATUS_DRAFT};
use a4n_core::models::project::novel_project;
use a4n_engine::llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, ModelCapabilities,
};
use a4n_engine::prompt::engine::PromptEntry;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use futures::stream;
use sea_orm::*;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

struct StaticLlmProvider {
    reply: String,
}

#[async_trait::async_trait]
impl LlmProvider for StaticLlmProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        Ok(ChatResponse {
            message: ChatMessage::assistant(self.reply.clone()),
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
    {
        Ok(Box::pin(stream::iter(vec![Ok(ChatChunk {
            content: Some(self.reply.clone()),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            index: 0,
            usage: None,
        })])))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    fn model_name(&self) -> &str {
        "static-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 8192,
            max_output_tokens: 1024,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn test_service(db: Arc<DatabaseConnection>) -> ChapterService {
    ChapterService::new(db, Arc::new(BroadcastEventBus::new(16)))
}

fn writing_prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::with_defaults(vec![PromptEntry {
        name: "writing".to_string(),
        title: "writing".to_string(),
        content: "系统提示词".to_string(),
        description: None,
        tags: vec![],
        category: "writing".to_string(),
        dependencies: vec![],
        is_modified: false,
        version: "1.0".to_string(),
        modified_at: chrono::Utc::now().naive_utc(),
        used_by: Vec::new(),
        path: None,
    }])
    .unwrap()
}

async fn insert_project(db: &DatabaseConnection) -> novel_project::Model {
    let now = fixed_time();
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("章节清理测试".to_string()),
        description: Set("验证空章节不会残留".to_string()),
        project_type: Set("novel_long".to_string()),
        status: Set("writing".to_string()),
        target_chapters: Set(10),
        current_chapter: Set(0),
        total_chapters: Set(Some(10)),
        chapters_per_part: Set(Some(5)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = fixed_time();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("有效章节".to_string())),
        content: Set("已有正文。".to_string()),
        word_count: Set(5),
        status: Set(STATUS_DRAFT.to_string()),
        summary: Set(Some("已有摘要。".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

#[test]
fn ensure_chapter_response_not_truncated_rejects_length_finish_reason() {
    let err = ensure_chapter_response_not_truncated(Some("length"), "第 1 章正文")
        .expect_err("length finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}

#[test]
fn ensure_chapter_response_not_truncated_rejects_max_tokens_finish_reason() {
    let err = ensure_chapter_response_not_truncated(Some("max_tokens"), "第 1 章正文")
        .expect_err("max_tokens finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}

#[tokio::test]
async fn failed_new_chapter_stream_generation_does_not_create_empty_chapter_row() {
    let db = Arc::new(setup_db().await);
    let service = test_service(db.clone());
    let project = insert_project(db.as_ref()).await;
    let prompt_engine = writing_prompt_engine();
    let llm = StaticLlmProvider {
        reply: " \n\t ".to_string(),
    };
    let workflow_run = WorkflowRunService::new(db.clone())
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("chapter_generation:{}:{}", project.id, 1),
            workflow_kind: "chapter_generation".to_string(),
            project_id: Some(project.id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("stream".to_string()),
            input_fingerprint_json: None,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("chapter_generation:chapter:1".to_string()),
            active_task_id: Some("test-chapter-generation".to_string()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .unwrap();

    let variables = HashMap::new();
    let err = service
        .generate_stream(super::generation::ChapterGenerateStreamInput {
            project_id: project.id,
            chapter_number: 1,
            chapter_outline: "第 1 章大纲",
            context_before: None,
            llm: &llm,
            prompt_engine: &prompt_engine,
            variables: &variables,
            workflow_run_id: workflow_run.id,
            cancel_token: None,
            on_chunk: |_delta, _index| {},
        })
        .await
        .expect_err("invalid LLM response must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(chapters.is_empty());
}

#[tokio::test]
async fn save_content_edit_rejects_empty_content_without_changing_chapter() {
    let db = Arc::new(setup_db().await);
    let service = test_service(db.clone());
    let project = insert_project(db.as_ref()).await;
    let chapter = insert_chapter(db.as_ref(), project.id).await;

    let err = service
        .save_content_edit(chapter.id, " \n\t ", None)
        .await
        .expect_err("blank content must fail");

    assert!(matches!(err, AppError::InvalidArgument(message) if message == "章节正文不能为空"));
    let stored = chapter::Entity::find_by_id(chapter.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("chapter must still exist");
    assert_eq!(stored.content, chapter.content);
    assert_eq!(stored.word_count, chapter.word_count);
    assert_eq!(stored.status, chapter.status);
}
