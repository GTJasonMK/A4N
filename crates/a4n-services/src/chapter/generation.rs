use super::{ChapterService, ensure_chapter_response_not_truncated};
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use super::generation_lifecycle::ChapterGenerationTrace;
use super::generation_prompt::build_chapter_generation_request;
use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};

pub struct ChapterGenerateStreamInput<'a, F>
where
    F: FnMut(&str, u32),
{
    pub project_id: i64,
    pub chapter_number: i32,
    pub chapter_outline: &'a str,
    pub context_before: Option<&'a str>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, serde_json::Value>,
    pub workflow_run_id: i64,
    pub cancel_token: Option<CancellationToken>,
    pub on_chunk: F,
}

impl ChapterService {
    /// 流式生成章节
    pub async fn generate_stream<F>(
        &self,
        input: ChapterGenerateStreamInput<'_, F>,
    ) -> Result<chapter::Model, AppError>
    where
        F: FnMut(&str, u32),
    {
        let ChapterGenerateStreamInput {
            project_id,
            chapter_number,
            chapter_outline,
            context_before,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
            cancel_token,
            mut on_chunk,
        } = input;
        use futures::StreamExt;
        let session = self.begin_generation(project_id, chapter_number).await?;
        let prompt_content = match prompt_engine.render("writing", variables, Some(project_id)) {
            Ok(prompt_content) => prompt_content,
            Err(err) => {
                self.restore_generation_status(session).await;
                return Err(err);
            }
        };
        let mut request = match build_chapter_generation_request(
            project_id,
            chapter_number,
            prompt_content,
            chapter_outline,
            context_before,
        ) {
            Ok(request) => request,
            Err(err) => {
                self.restore_generation_status(session).await;
                return Err(err);
            }
        };
        let call_key = format!("chapter_generation:chapter:{chapter_number}");
        let input_fingerprint_json =
            chapter_generation_call_fingerprint_json(project_id, chapter_number, &request)?;
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        let request_trace = request.clone();
        let model_name = llm.model_name().to_string();

        let mut restored_from_checkpoint = false;
        let (accumulated, finish_reason) = if let Some(checkpoint) =
            load_completed_chapter_generation_checkpoint(
                self,
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await?
        {
            ensure_chapter_response_not_truncated(
                checkpoint.finish_reason.as_deref(),
                &format!("第 {chapter_number} 章正文"),
            )?;
            let validated = self.validate_generated_chapter_for_checkpoint(
                &session,
                &checkpoint.response_content,
                chapter_outline,
            )?;
            if checkpoint.parsed_content != validated.normalized_content {
                self.restore_generation_status(session).await;
                return Err(AppError::InvalidArgument(format!(
                    "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                )));
            }
            if !checkpoint.response_content.trim().is_empty() {
                on_chunk(&checkpoint.response_content, 0);
            }
            restored_from_checkpoint = true;
            (checkpoint.response_content, checkpoint.finish_reason)
        } else {
            let stream_result = if let Some(cancel_token) = cancel_token.as_ref() {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        self.restore_generation_status(session).await;
                        return Err(AppError::Cancelled);
                    }
                    result = llm.chat_stream(request) => result,
                }
            } else {
                llm.chat_stream(request).await
            };
            let mut stream = match stream_result {
                Ok(stream) => stream,
                Err(err) => {
                    self.restore_generation_status(session).await;
                    return Err(err);
                }
            };

            let mut accumulated = String::new();
            let mut idx: u32 = 0;
            let mut finish_reason: Option<String> = None;
            loop {
                let item = if let Some(cancel_token) = cancel_token.as_ref() {
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            self.restore_generation_status(session).await;
                            return Err(AppError::Cancelled);
                        }
                        item = stream.next() => item,
                    }
                } else {
                    stream.next().await
                };
                let Some(item) = item else {
                    break;
                };
                let chunk = match item {
                    Ok(chunk) => chunk,
                    Err(err) => {
                        self.restore_generation_status(session).await;
                        return Err(err);
                    }
                };
                if let Some(delta) = chunk.content.as_deref()
                    && !delta.is_empty()
                {
                    accumulated.push_str(delta);
                    on_chunk(delta, idx);
                    idx += 1;
                }
                if chunk.finish_reason.is_some() {
                    finish_reason = chunk.finish_reason;
                    break;
                }
            }
            (accumulated, finish_reason)
        };
        if let Err(err) = ensure_chapter_response_not_truncated(
            finish_reason.as_deref(),
            &format!("第 {chapter_number} 章正文"),
        ) {
            self.restore_generation_status(session).await;
            return Err(err);
        }
        let validated =
            self.validate_generated_chapter_for_checkpoint(&session, &accumulated, chapter_outline);
        let validated = match validated {
            Ok(validated) => validated,
            Err(err) => {
                self.restore_generation_status(session).await;
                return Err(err);
            }
        };
        if !restored_from_checkpoint
            && let Err(err) =
                save_completed_chapter_generation_checkpoint(ChapterGenerationCheckpointSaveInput {
                    service: self,
                    workflow_run_id,
                    call_key,
                    chapter_number,
                    input_fingerprint_json,
                    response_content: &accumulated,
                    parsed_content: &validated.normalized_content,
                    finish_reason: finish_reason.as_deref(),
                })
                .await
        {
            self.restore_generation_status(session).await;
            return Err(err);
        }

        match self
            .finish_generated_chapter(
                &session,
                &accumulated,
                chapter_outline,
                "流式生成",
                Some(ChapterGenerationTrace {
                    request: request_trace,
                    finish_reason,
                    model_name,
                }),
            )
            .await
        {
            Ok(chapter) => Ok(chapter),
            Err(err) => {
                self.restore_generation_status(session).await;
                Err(err)
            }
        }
    }
}

struct ChapterGenerationCheckpoint {
    response_content: String,
    parsed_content: String,
    finish_reason: Option<String>,
}

fn chapter_generation_call_fingerprint_json(
    project_id: i64,
    chapter_number: i32,
    request: &a4n_engine::llm::provider::ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "chapter_generation",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

async fn load_completed_chapter_generation_checkpoint(
    service: &ChapterService,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<ChapterGenerationCheckpoint>, AppError> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        service.db.clone(),
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.response_content 必须是字符串"))
        })?;
    let parsed_content = checkpoint_json
        .get("parsed_output")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.parsed_output.content 必须是字符串"))
        })?;
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{call_key}.finish_reason 必须是字符串或 null"
                    ))
                })?
                .to_string(),
        ),
    };
    Ok(Some(ChapterGenerationCheckpoint {
        response_content,
        parsed_content,
        finish_reason,
    }))
}

async fn save_completed_chapter_generation_checkpoint(
    input: ChapterGenerationCheckpointSaveInput<'_>,
) -> Result<(), AppError> {
    let ChapterGenerationCheckpointSaveInput {
        service,
        workflow_run_id,
        call_key,
        chapter_number,
        input_fingerprint_json,
        response_content,
        parsed_content,
        finish_reason,
    } = input;
    save_completed_llm_call_checkpoint(
        service.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: chapter_number,
            input_fingerprint_json,
            checkpoint_json: json!({
                "output": {
                    "content": parsed_content,
                },
                "parsed_output": {
                    "content": parsed_content,
                },
                "response_content": response_content,
                "usage": null,
                "finish_reason": finish_reason,
            }),
            artifacts_json: None,
        },
    )
    .await
}

struct ChapterGenerationCheckpointSaveInput<'a> {
    service: &'a ChapterService,
    workflow_run_id: i64,
    call_key: String,
    chapter_number: i32,
    input_fingerprint_json: Value,
    response_content: &'a str,
    parsed_content: &'a str,
    finish_reason: Option<&'a str>,
}
