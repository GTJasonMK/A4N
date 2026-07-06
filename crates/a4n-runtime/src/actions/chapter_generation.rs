mod generation_context;

use crate::AppState;
use crate::actions::access::verify_project_editor_string;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use a4n_core::error::AppError;
use a4n_services::project::PROJECT_KIND_TEXT;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde::Deserialize;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use generation_context::{
    GenerationRagContext, PreparedGenerationRagEmbeddingRequest, build_generation_rag_context,
    build_generation_rag_context_with_embeddings, find_generation_outline,
    merge_generation_outline_text, non_empty_trimmed, prepare_generation_rag_embedding_request,
    required_generation_chapter_title, resolve_generation_reference_context,
    resolve_generation_word_count_range, upsert_chapter_generation_variables,
};

pub type ChapterGenerationProgress<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
pub type ChapterGenerationChunk<'a> = &'a mut (dyn FnMut(&str, u32) -> Result<(), AppError> + Send);

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterStreamInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: Option<String>,
    pub writing_notes: Option<String>,
    pub min_word_count: Option<i32>,
    pub max_word_count: Option<i32>,
    pub client_rag_context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewChapterPromptInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub writing_notes: Option<String>,
    pub min_word_count: Option<i32>,
    pub max_word_count: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinishChapterGenerationRagContextInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub writing_notes: Option<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_vectors: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterGenerationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

struct PreparedChapterGeneration {
    chapter_title: String,
    outline_text: String,
    context_before: Option<String>,
    word_count_range: generation_context::ChapterGenerationWordCountRange,
    use_rag: bool,
    variables: HashMap<String, serde_json::Value>,
}

fn chapter_generation_workflow_fingerprint(
    project_id: i64,
    chapter_number: i32,
    prepared: &PreparedChapterGeneration,
) -> serde_json::Value {
    serde_json::json!({
        "workflow": "chapter_generation",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapter_title": prepared.chapter_title,
        "outline_text": prepared.outline_text,
        "context_before": prepared.context_before,
        "word_count_range": {
            "min": prepared.word_count_range.min,
            "max": prepared.word_count_range.max,
        },
        "use_rag": prepared.use_rag,
        "variables": prepared.variables,
    })
}

pub async fn generate_chapter_stream(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterStreamInput,
    mut progress: Option<ChapterGenerationProgress<'_>>,
    mut chunk: Option<ChapterGenerationChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<String, String> {
    generate_chapter_stream_task_scope(state, user_id, &input).await?;
    let task_id = chapter_stream_task_id(input.project_id, input.chapter_number, input.task_id)?;
    let progress_total = 1;

    report_progress(&mut progress, 0, progress_total, "正在读取章节大纲")?;
    check_cancelled(&cancel_token)?;
    let prepared = prepare_chapter_generation(
        state,
        input.project_id,
        input.chapter_number,
        input.writing_notes.as_deref(),
        input.min_word_count,
        input.max_word_count,
        input.client_rag_context.as_deref(),
        &mut progress,
        progress_total,
        Some(&cancel_token),
    )
    .await?;

    report_progress(&mut progress, 0, progress_total, "正在流式生成正文")?;
    check_cancelled(&cancel_token)?;
    let workflow_fingerprint =
        chapter_generation_workflow_fingerprint(input.project_id, input.chapter_number, &prepared);
    let call_key = format!("chapter_generation:chapter:{}", input.chapter_number);
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
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
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key),
            active_task_id: Some(task_id.clone()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let mut display_chunk_index = 0u32;
    let chunk_cancel_token = cancel_token.clone();
    let result = state
        .chapter
        .generate_stream(a4n_services::chapter::ChapterGenerateStreamInput {
            project_id: input.project_id,
            chapter_number: input.chapter_number,
            chapter_outline: &prepared.outline_text,
            context_before: prepared.context_before.as_deref(),
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &prepared.variables,
            workflow_run_id: workflow_run.id,
            cancel_token: Some(cancel_token.clone()),
            on_chunk: move |delta, _idx| {
                if delta.is_empty() {
                    return;
                }
                if let Some(chunk) = chunk.as_deref_mut()
                    && chunk(delta, display_chunk_index).is_err()
                {
                    chunk_cancel_token.cancel();
                    return;
                }
                display_chunk_index = display_chunk_index.saturating_add(1);
            },
        })
        .await;

    match result {
        Ok(chapter) => {
            report_progress(&mut progress, 1, progress_total, "正文已保存")?;
            let postprocess_result =
                crate::workflow::postprocess_chapter_after_content_change(state, chapter.id)
                    .await
                    .map_err(|err| err.to_string());
            match postprocess_result {
                Ok(()) => {
                    state
                        .workflow_run
                        .mark_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
                        .await
                        .map_err(|err| err.to_string())?;
                    Ok(task_id)
                }
                Err(err) => {
                    state
                        .workflow_run
                        .mark_status(workflow_run.id, WORKFLOW_STATUS_FAILED, true, Some(&err))
                        .await
                        .map_err(|status_err| {
                            format!("{err}; 记录章节生成工作流失败状态失败: {status_err}")
                        })?;
                    Err(err)
                }
            }
        }
        Err(err) => {
            let message = err.to_string();
            state
                .workflow_run
                .mark_status(
                    workflow_run.id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(&message),
                )
                .await
                .map_err(|status_err| {
                    format!("{message}; 记录章节生成工作流失败状态失败: {status_err}")
                })?;
            Err(message)
        }
    }
}

pub async fn generate_chapter_stream_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterStreamInput,
) -> Result<ChapterGenerationTaskScope, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    ensure_text_source_project(state, input.project_id, "生成章节正文").await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    Ok(ChapterGenerationTaskScope {
        project_id: input.project_id,
        total: 1,
    })
}

pub async fn preview_chapter_prompt(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<ModelRequestPreview, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    ensure_text_source_project(state, input.project_id, "预览章节正文生成请求").await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    let prepared = prepare_chapter_generation(
        state,
        input.project_id,
        input.chapter_number,
        input.writing_notes.as_deref(),
        input.min_word_count,
        input.max_word_count,
        None,
        &mut None,
        1,
        None,
    )
    .await?;

    let system_prompt = state
        .engine
        .render("writing", &prepared.variables, Some(input.project_id))
        .map_err(|err| format!("渲染 writing 提示词失败: {err}"))?;
    let request = a4n_services::chapter::build_chapter_generation_request(
        input.project_id,
        input.chapter_number,
        system_prompt,
        &prepared.outline_text,
        prepared.context_before.as_deref(),
    )
    .map_err(|err| format!("构建章节生成请求失败: {err}"))?;
    let note = format!(
        "章节：{}；字数：{}-{}；检索参考：{}",
        prepared.chapter_title,
        prepared.word_count_range.min,
        prepared.word_count_range.max,
        if prepared.use_rag {
            "启用"
        } else {
            "未启用"
        }
    );
    Ok(model_request_preview(
        "chapter_generation",
        format!("第 {} 章生成请求预览", input.chapter_number),
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "chapter_generation".to_string(),
                    title: "章节正文生成 LLM 请求".to_string(),
                    prompt_name: Some("writing".to_string()),
                    state: "ready".to_string(),
                    note: Some(note),
                    request,
                },
            )
            .await?,
        ],
    ))
}

pub async fn prepare_chapter_generation_rag_embedding(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    ensure_text_source_project(state, input.project_id, "准备章节生成 RAG 检索").await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    if input.chapter_number <= 1 {
        let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
            .await
            .map_err(|err| err.to_string())?;
        return Ok(PreparedGenerationRagEmbeddingRequest {
            query_texts: Vec::new(),
            embedding_identity,
        });
    }
    let outline = find_generation_outline(state, input.project_id, input.chapter_number)
        .await?
        .ok_or_else(|| {
            format!(
                "项目 {} 第 {} 章大纲未找到",
                input.project_id, input.chapter_number
            )
        })?;
    let writing_instruction = input.writing_notes.as_deref().and_then(non_empty_trimmed);
    let chapter_title =
        required_generation_chapter_title(input.chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        input.chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    prepare_generation_rag_embedding_request(
        state,
        input.project_id,
        input.chapter_number,
        Some(&outline),
        &rag_query_outline_text,
        writing_instruction,
    )
    .await
}

pub async fn finish_chapter_generation_rag_context(
    state: &AppState,
    user_id: i64,
    input: FinishChapterGenerationRagContextInput,
) -> Result<GenerationRagContext, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    ensure_text_source_project(state, input.project_id, "构建章节生成 RAG 上下文").await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    if input.chapter_number <= 1 {
        return Ok(GenerationRagContext::default());
    }
    let outline = find_generation_outline(state, input.project_id, input.chapter_number)
        .await?
        .ok_or_else(|| {
            format!(
                "项目 {} 第 {} 章大纲未找到",
                input.project_id, input.chapter_number
            )
        })?;
    let writing_instruction = input.writing_notes.as_deref().and_then(non_empty_trimmed);
    let chapter_title =
        required_generation_chapter_title(input.chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        input.chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    build_generation_rag_context_with_embeddings(
        state,
        input.project_id,
        input.chapter_number,
        Some(&outline),
        &rag_query_outline_text,
        writing_instruction,
        input.embedding_identity,
        input.query_vectors,
    )
    .await
}

async fn prepare_chapter_generation(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<&str>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag_context: Option<&str>,
    progress: &mut Option<ChapterGenerationProgress<'_>>,
    progress_total: u32,
    cancel_token: Option<&CancellationToken>,
) -> Result<PreparedChapterGeneration, String> {
    let outline = find_generation_outline(state, project_id, chapter_number)
        .await?
        .ok_or_else(|| format!("项目 {project_id} 第 {chapter_number} 章大纲未找到"))?;
    let writing_instruction = writing_notes.and_then(non_empty_trimmed);
    let word_count_range =
        resolve_generation_word_count_range(Some(&outline), min_word_count, max_word_count)?;
    let chapter_title = required_generation_chapter_title(chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    let mut outline_text = merge_generation_outline_text(
        chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        Some(word_count_range),
    )?;

    report_progress(progress, 0, progress_total, "正在准备章节参考资料")?;
    check_optional_cancelled(cancel_token)?;
    let reference_context =
        resolve_generation_reference_context(state, project_id, chapter_number, Some(&outline))
            .await?;
    if let Some(reference_context) = reference_context.as_deref() {
        outline_text.push_str("\n\n");
        outline_text.push_str(reference_context);
    }

    report_progress(progress, 0, progress_total, "正在检索相关 RAG 上下文")?;
    check_optional_cancelled(cancel_token)?;
    let use_rag = chapter_number > 1;
    let rag_result = if use_rag {
        if let Some(context) = client_rag_context
            .map(str::trim)
            .filter(|context| !context.is_empty())
        {
            Some(GenerationRagContext {
                text: Some(context.to_string()),
                statistics: Default::default(),
            })
        } else {
            Some(
                build_generation_rag_context(
                    state,
                    project_id,
                    chapter_number,
                    Some(&outline),
                    &rag_query_outline_text,
                    writing_instruction,
                )
                .await
                .map_err(|err| format!("构建章节生成检索参考失败: {err}"))?,
            )
        }
    } else {
        None
    };
    let rag_context = rag_result.and_then(|context| context.text);
    if let Some(rag_context) = rag_context.as_deref() {
        outline_text.push_str("\n\n");
        outline_text.push_str(rag_context);
    }

    report_progress(progress, 0, progress_total, "正在收集前文上下文")?;
    check_optional_cancelled(cancel_token)?;
    let context_before =
        crate::workflow::resolve_chapter_context_before(state, project_id, chapter_number, None)
            .await
            .map_err(|err| format!("构建章节前文上下文失败: {err}"))?;

    let mut variables = HashMap::new();
    upsert_chapter_generation_variables(
        &mut variables,
        chapter_number,
        &chapter_title,
        &outline_text,
        writing_instruction,
        reference_context.as_deref(),
        rag_context.as_deref(),
        word_count_range,
    );

    Ok(PreparedChapterGeneration {
        chapter_title,
        outline_text,
        context_before,
        word_count_range,
        use_rag,
        variables,
    })
}

async fn ensure_text_source_project(
    state: &AppState,
    project_id: i64,
    operation: &str,
) -> Result<(), String> {
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(format!("{operation}只能作用于文本项目"));
    }
    Ok(())
}

pub fn chapter_stream_task_id(
    project_id: i64,
    chapter_number: i32,
    provided: Option<String>,
) -> Result<String, String> {
    let Some(candidate) = provided.map(|id| id.trim().to_string()) else {
        return Ok(format!(
            "chapter-{project_id}-{chapter_number}-{}",
            chrono::Utc::now().timestamp_millis()
        ));
    };
    if candidate.is_empty() {
        return Err("task_id 不能为空".to_string());
    }
    if candidate.len() > 160 {
        return Err("task_id 长度不能超过 160 个字符".to_string());
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err("task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string());
    }
    Ok(candidate)
}

fn report_progress(
    progress: &mut Option<ChapterGenerationProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn check_optional_cancelled(cancel_token: Option<&CancellationToken>) -> Result<(), String> {
    if let Some(cancel_token) = cancel_token {
        check_cancelled(cancel_token)?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_stream_task_id_generates_id_when_absent() {
        let task_id = chapter_stream_task_id(7, 12, None).expect("generated task id");

        assert!(task_id.starts_with("chapter-7-12-"));
    }

    #[test]
    fn chapter_stream_task_id_accepts_trimmed_safe_id() {
        let task_id = chapter_stream_task_id(7, 12, Some("  chapter:7_12.alpha-1  ".to_string()))
            .expect("provided task id");

        assert_eq!(task_id, "chapter:7_12.alpha-1");
    }

    #[test]
    fn chapter_stream_task_id_rejects_invalid_provided_id() {
        assert!(chapter_stream_task_id(7, 12, Some("   ".to_string())).is_err());
        assert!(chapter_stream_task_id(7, 12, Some("章节-7-12".to_string())).is_err());
        assert!(chapter_stream_task_id(7, 12, Some("a".repeat(161))).is_err());
    }
}
