use super::messages::{
    OptimizeIssue, build_content_optimization_chat_user_message,
    build_content_optimization_patch_user_message, build_content_optimization_user_message,
    is_content_optimization_analysis_prompt, materialize_optimization_issue_evidence,
    optimization_chat_history, parse_content_optimization_issues, parse_optimization_issues,
};
use super::*;
use a4n_core::models::ai::optimization_turn;
use a4n_core::models::project::novel_project;
use a4n_engine::llm::provider::{
    ChatChunk, ChatMessage, ChatResponse, MessageRole, ModelCapabilities,
};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use futures::stream;
use std::collections::VecDeque;
use std::path::Path;
use std::pin::Pin;
use std::sync::Mutex;

mod analysis_flow;
mod chat_flow;
mod message_contract;
mod patch_flow;
mod runtime_events;
mod session_flow;

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

struct SequenceLlmProvider {
    replies: Mutex<VecDeque<String>>,
    requests: Mutex<Vec<ChatRequest>>,
}

impl SequenceLlmProvider {
    fn new(replies: Vec<String>) -> Self {
        Self {
            replies: Mutex::new(VecDeque::from(replies)),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<ChatRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl LlmProvider for SequenceLlmProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.requests.lock().unwrap().push(request);
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::InvalidArgument("测试 LLM 回复队列已空".to_string()))?;
        Ok(ChatResponse {
            message: ChatMessage::assistant(reply),
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
    {
        self.requests.lock().unwrap().push(request);
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::InvalidArgument("测试 LLM 回复队列已空".to_string()))?;
        Ok(Box::pin(stream::iter(vec![Ok(ChatChunk {
            content: Some(reply),
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
        "sequence-test-llm"
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

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
}

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 31)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

async fn insert_test_project_and_chapter(
    db: &DatabaseConnection,
) -> (novel_project::Model, chapter::Model) {
    let now = fixed_time();
    let project = novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("Agent 可见性测试".to_string()),
        description: Set("测试普通聊天链路".to_string()),
        project_type: Set("novel_long".to_string()),
        status: Set("writing".to_string()),
        target_chapters: Set(10),
        current_chapter: Set(1),
        total_chapters: Set(Some(10)),
        chapters_per_part: Set(Some(5)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(Some("主角在旧城调查真相。".to_string())),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("旧城来信".to_string())),
        content: Set("第一段正文。主角收到旧信，决定进入旧城。".to_string()),
        word_count: Set(22),
        status: Set("completed".to_string()),
        summary: Set(Some("主角收到旧信并决定调查。".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    (project, chapter)
}

async fn insert_test_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
    title: &str,
) -> chapter::Model {
    let now = fixed_time();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(title.to_string())),
        content: Set(format!("第{chapter_number}章正文。主角继续调查旧城。")),
        word_count: Set(18),
        status: Set("completed".to_string()),
        summary: Set(Some(format!("第{chapter_number}章摘要。"))),
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

fn analysis_variables(
    instruction: Option<&str>,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut variables = std::collections::HashMap::from([
        (
            "mode".to_string(),
            serde_json::Value::String("full".to_string()),
        ),
        (
            "agent_permissions".to_string(),
            OptimizeAgentPermissions::default().as_value(),
        ),
    ]);
    if let Some(instruction) = instruction {
        variables.insert(
            "agent_instruction".to_string(),
            serde_json::Value::String(instruction.to_string()),
        );
    }
    variables
}

fn permissions_with_automatic_writeback(enabled: bool) -> OptimizeAgentPermissions {
    OptimizeAgentPermissions {
        automatic_writeback: enabled,
        ..OptimizeAgentPermissions::default()
    }
}

fn visible_test_issues() -> serde_json::Value {
    json!([{
        "id": "issue-plot-1",
        "tool": "edit_article",
        "arguments": {
            "evidence": "决定进入旧城",
            "goal": "补足主角进入旧城的行动动机"
        },
        "description": "主角进入旧城的动机铺垫不足",
        "severity": "major"
    }])
}

fn model_final_test_issues() -> serde_json::Value {
    json!([{
        "id": "issue-plot-1",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": "ch1:s2:f2",
            "goal": "补足主角进入旧城的行动动机"
        },
        "description": "主角进入旧城的动机铺垫不足",
        "severity": "major"
    }])
}

async fn insert_waiting_session_with_issues(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> optimization_session::Model {
    let now = fixed_time();
    optimization_session::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        status: Set("waiting_user".to_string()),
        issues: Set(Some(visible_test_issues())),
        agent_permissions: Set(Some(OptimizeAgentPermissions::default().as_value())),
        agent_instruction: Set(None),
        fixes_applied: Set(0),
        summary: Set(Some("发现 1 个问题".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_optimize_workflow_run(
    db: Arc<DatabaseConnection>,
    session: &optimization_session::Model,
    workflow_kind: &str,
    mode: &str,
) -> i64 {
    let run = crate::workflow::WorkflowRunService::new(db)
        .upsert_run(crate::workflow::UpsertWorkflowRunInput {
            business_key: format!("test:{workflow_kind}:{}:{mode}", session.id),
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(session.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("optimization_session".to_string()),
            scope_id: Some(session.id),
            chapter_id: Some(session.chapter_id),
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(json!({
                "workflow": workflow_kind,
                "session_id": session.id,
                "mode": mode,
            })),
            status: crate::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(format!("test:{workflow_kind}:{}:{mode}", session.id)),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .unwrap();
    run.id
}

fn assert_message_contains(
    messages: &[optimization_session_message::Model],
    role: &str,
    needle: &str,
) {
    assert!(
        messages
            .iter()
            .any(|message| message.role == role && message.content.contains(needle)),
        "expected role={role} message containing {needle:?}; got: {:?}",
        messages
            .iter()
            .map(|message| (&message.role, &message.content))
            .collect::<Vec<_>>()
    );
}

fn assert_no_message_contains(
    messages: &[optimization_session_message::Model],
    role: &str,
    needle: &str,
) {
    assert!(
        !messages
            .iter()
            .any(|message| message.role == role && message.content.contains(needle)),
        "did not expect role={role} message containing {needle:?}; got: {:?}",
        messages
            .iter()
            .map(|message| (&message.role, &message.content))
            .collect::<Vec<_>>()
    );
}

fn request_user_content(request: &ChatRequest) -> &str {
    request
        .messages
        .iter()
        .rev()
        .find(|message| matches!(&message.role, MessageRole::User))
        .map(|message| message.content.as_str())
        .expect("request should contain user message")
}
