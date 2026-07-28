use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use a4n_api_contract::{
    chapter::StartChapterGenerationInput, generation_tasks::GenerationTaskHandle,
};
use a4n_core::{error::AppError, models::chapter::chapter_outline};
use a4n_db::workflow;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{
    GenerationError, GenerationFinishReason, GenerationResultRef, ProgressEvent, ProgressPhaseKind,
    ProgressSeverity, StreamTextHash,
};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    chapter_content::ChapterDerivedCleanupHostPort,
    collaboration::ProjectAccessRole,
    context::GenerationTaskExecution,
    generation::{
        checkpoint::{
            DatabaseGenerationSequenceLeaseReplenisher, GenerationCheckpointMetadata,
            GenerationCheckpointWriter,
        },
        events::{
            GenerationEventPublisher, GenerationSessionError, SessionGenerationEventPublisher,
        },
        generation_finish_reason_text as shared_generation_finish_reason_text,
        generation_public_error,
        stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
        terminal::{CompletionDescriptor, GenerationTerminalError},
    },
    workflow::workflow_checkpoint_contract_app_error,
};

use super::service::{
    ChapterGenerationService, ChapterGenerationSession,
    generation::checkpoint::{
        ChapterGenerationCheckpointSaveInput, chapter_generation_call_fingerprint_json,
        save_completed_chapter_generation_checkpoint,
    },
};

use super::{
    ChapterGenerationWordCountRange, GenerationRagContext, PreparedGenerationRagSearch,
    merge_generation_outline_text, non_empty_trimmed, required_generation_chapter_title,
    resolve_generation_word_count_range, upsert_chapter_generation_variables,
};

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewChapterGenerationInput {
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

#[derive(Debug, Clone, Serialize)]
pub struct PreparedGenerationRagEmbeddingRequest {
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
}

pub struct ChapterGenerationApplicationContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub runtime_embedding_identity: Option<EmbeddingModelIdentity>,
}

pub struct ChapterGenerationRequestPreview {
    pub title: String,
    pub note: String,
    pub request: ChatRequest,
}

struct PreparedChapterGeneration {
    chapter_title: String,
    outline_text: String,
    context_before: Option<String>,
    word_count_range: ChapterGenerationWordCountRange,
    use_rag: bool,
    variables: HashMap<String, Value>,
}

struct PrepareChapterGenerationInput<'a> {
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<&'a str>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag_context: Option<&'a str>,
    runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    cancel_token: Option<&'a CancellationToken>,
}

impl OrchestratorContext {
    /// Runs a reserved chapter generation attempt through the canonical
    /// generation session/event/terminal path.
    pub async fn run_chapter_generation_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: StartChapterGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let replenisher = Arc::new(DatabaseGenerationSequenceLeaseReplenisher::new(
            self.database.shared_connection(),
        ));
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
        let chapter_service = self.chapter_generation_service();

        if let Err(error) = publisher
            .progress(chapter_progress(
                "chapter_prepare",
                ProgressPhaseKind::Prepare,
                "正在准备章节生成上下文",
            ))
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                None,
                None,
                generation_error_from_session(error),
            )
            .await;
        }

        let chapter_session = match chapter_service
            .begin_generation(input.project_id, input.chapter_number)
            .await
        {
            Ok(session) => Arc::new(session),
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    None,
                    None,
                    generation_error_from_app(
                        "generation_validation_failed",
                        "chapter_prepare",
                        error,
                    ),
                )
                .await;
            }
        };

        let prepared = match self
            .prepare_chapter_generation_application(PrepareChapterGenerationInput {
                project_id: input.project_id,
                chapter_number: input.chapter_number,
                writing_notes: input.writing_notes.as_deref(),
                min_word_count: input.min_word_count,
                max_word_count: input.max_word_count,
                client_rag_context: input.client_rag_context.as_deref(),
                runtime_embedding_identity: context.runtime_embedding_identity,
                cancel_token: Some(&cancel_token),
            })
            .await
        {
            Ok(prepared) => prepared,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    None,
                    generation_error_from_text(
                        "generation_request_invalid",
                        "chapter_prepare",
                        error,
                    ),
                )
                .await;
            }
        };

        if let Err(error) = publisher
            .progress(chapter_progress(
                "chapter_prompt",
                ProgressPhaseKind::Prepare,
                "正在渲染章节生成提示词",
            ))
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(Arc::clone(&chapter_session)),
                None,
                generation_error_from_session(error),
            )
            .await;
        }
        let system_prompt = match context
            .prompt_renderer
            .render_prompt("writing", &prepared.variables, Some(input.project_id))
            .await
        {
            Ok(prompt) => prompt,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    None,
                    generation_error_from_app(
                        "generation_request_invalid",
                        "chapter_prompt",
                        error,
                    ),
                )
                .await;
            }
        };
        let mut request = match a4n_prompt::chapter_generation::build_chapter_generation_request(
            input.project_id,
            input.chapter_number,
            system_prompt,
            &prepared.outline_text,
            prepared.context_before.as_deref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    None,
                    generation_error_from_text(
                        "generation_request_invalid",
                        "chapter_prompt",
                        error.to_string(),
                    ),
                )
                .await;
            }
        };
        let workflow_run_id = match self
            .start_chapter_generation_workflow(super::StartChapterGenerationWorkflowInput {
                project_id: input.project_id,
                chapter_number: input.chapter_number,
                task_id: &handle.task_id,
                chapter_title: &prepared.chapter_title,
                outline_text: &prepared.outline_text,
                context_before: prepared.context_before.as_deref(),
                word_count_range: prepared.word_count_range,
                use_rag: prepared.use_rag,
                variables: &prepared.variables,
            })
            .await
        {
            Ok(workflow_run_id) => workflow_run_id,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    None,
                    generation_error_from_text(
                        "generation_persist_failed",
                        "chapter_workflow",
                        error,
                    ),
                )
                .await;
            }
        };
        let call_key = format!("chapter_generation:chapter:{}", input.chapter_number);
        let input_fingerprint_json = match chapter_generation_call_fingerprint_json(
            input.project_id,
            input.chapter_number,
            &request,
        ) {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    Some(workflow_run_id),
                    generation_error_from_app(
                        "generation_request_invalid",
                        "chapter_prompt",
                        error,
                    ),
                )
                .await;
            }
        };
        let input_fingerprint_sha256 =
            match workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)
            {
                Ok(fingerprint) => fingerprint,
                Err(error) => {
                    return fail_chapter_generation_execution(
                        self,
                        &chapter_service,
                        execution,
                        Some(Arc::clone(&chapter_session)),
                        Some(workflow_run_id),
                        generation_error_from_app(
                            "generation_request_invalid",
                            "chapter_prompt",
                            error,
                        ),
                    )
                    .await;
                }
            };
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        let request_trace = request.clone();
        let model_name = self.llm_model_name();
        let checkpoint_writer = GenerationCheckpointWriter::new(self.database.shared_connection());
        let checkpoint_metadata = GenerationCheckpointMetadata {
            step_key: Some("chapter_generation".to_string()),
            call_key: Some(call_key.clone()),
            provider_model: Some(model_name.clone()),
            ..Default::default()
        };

        if let Err(error) = publisher
            .progress(chapter_progress(
                "chapter_stream",
                ProgressPhaseKind::Stream,
                "正在流式生成正文",
            ))
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(Arc::clone(&chapter_session)),
                Some(workflow_run_id),
                generation_error_from_session(error),
            )
            .await;
        }
        let output = match execute_llm_generation(
            self.llm_backend(),
            &publisher,
            LlmGenerationRequest {
                request,
                stream_id: "main".to_string(),
                cancellation: cancel_token.clone(),
                checkpoint: Some(LlmGenerationCheckpoint {
                    writer: &checkpoint_writer,
                    session: &session,
                    metadata: checkpoint_metadata.clone(),
                }),
                usage_base: None,
            },
        )
        .await
        {
            Ok(output) => output,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    Some(workflow_run_id),
                    error,
                )
                .await;
            }
        };

        if let Err(error) = checkpoint_writer
            .persist_if_due(&session, checkpoint_metadata, true)
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(Arc::clone(&chapter_session)),
                Some(workflow_run_id),
                generation_error_from_text(
                    "generation_checkpoint_unavailable",
                    "checkpoint",
                    error.to_string(),
                ),
            )
            .await;
        }

        let validated = match chapter_service.validate_generated_chapter_for_checkpoint(
            &chapter_session,
            &output.content,
            &prepared.outline_text,
        ) {
            Ok(validated) => validated,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    Some(workflow_run_id),
                    generation_error_from_app(
                        "generation_validation_failed",
                        "chapter_validate",
                        error,
                    ),
                )
                .await;
            }
        };
        let finish_reason_text = generation_finish_reason_text(output.finish_reason);
        if let Err(error) =
            save_completed_chapter_generation_checkpoint(ChapterGenerationCheckpointSaveInput {
                service: &chapter_service,
                workflow_run_id,
                call_key: call_key.clone(),
                chapter_number: input.chapter_number,
                input_fingerprint_json,
                response_content: &output.content,
                parsed_content: &validated.normalized_content,
                finish_reason: Some(&finish_reason_text),
            })
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(Arc::clone(&chapter_session)),
                Some(workflow_run_id),
                generation_error_from_app("generation_persist_failed", "chapter_checkpoint", error),
            )
            .await;
        }

        let chapter_id = Arc::new(AtomicI64::new(0));
        let chapter_id_slot = Arc::clone(&chapter_id);
        let finish_reason = output.finish_reason;
        let content = output.content;
        let outline_text = prepared.outline_text;
        let trace = crate::chapter_generation::service::ChapterGenerationTrace {
            request: request_trace,
            finish_reason: Some(finish_reason_text.clone()),
            model_name,
        };
        let completion_service = chapter_service.clone();
        let completion_chapter_session = Arc::clone(&chapter_session);
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let chapter_service = completion_service.clone();
                let chapter_session = Arc::clone(&completion_chapter_session);
                let content = content.clone();
                let outline_text = outline_text.clone();
                let trace = trace.clone();
                Box::pin(async move {
                    let chapter = chapter_service
                        .finish_generated_chapter_in_transaction(
                            &chapter_session,
                            &content,
                            &outline_text,
                            Some(trace),
                            tx,
                        )
                        .await?;
                    chapter_id_slot.store(chapter.id, Ordering::Release);
                    let sha256 = format!("{:x}", Sha256::digest(chapter.content.as_bytes()));
                    Ok(CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "chapter".to_string(),
                            id: chapter.id.to_string(),
                            revision: sha256.clone(),
                        }),
                        final_text_hashes: vec![StreamTextHash {
                            stream_id: "main".to_string(),
                            utf8_bytes: chapter.content.len() as u64,
                            sha256,
                        }],
                        final_usage: output.usage.clone(),
                        finish_reason: Some(finish_reason),
                        summary: Some(format!("第 {} 章正文已保存", chapter.chapter_number)),
                        batch_summary: None,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(chapter_session),
                Some(workflow_run_id),
                generation_error_from_terminal(error),
            )
            .await;
        }

        let task_id = execution.permit.task_id().to_string();
        if let Err(error) = execution
            .queue
            .complete_generation_task(execution.permit)
            .await
        {
            return Err(format!(
                "generation queue 完成投影失败 ({task_id}): {error}"
            ));
        }
        let chapter_id = chapter_id.load(Ordering::Acquire);
        if chapter_id <= 0 {
            return Err("generation completed 但缺少章节结果 id".to_string());
        }
        if let Err(error) = self
            .postprocess_chapter_after_content_change_validated(host_port, chapter_id)
            .await
        {
            let _ = self
                .fail_chapter_generation_workflow(workflow_run_id, &error.to_string())
                .await;
            tracing::error!(
                task_id = %handle.task_id,
                chapter_id,
                error = %error,
                "章节正文已完成，但章节后处理失败"
            );
            return Err(error.to_string());
        }
        self.complete_chapter_generation_workflow(workflow_run_id)
            .await?;
        Ok(())
    }

    pub async fn preview_chapter_generation_request_validated(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterGenerationInput,
    ) -> Result<ChapterGenerationRequestPreview, String> {
        self.validate_chapter_generation_application_access(
            user_id,
            input.project_id,
            input.chapter_number,
            "预览章节正文生成请求",
        )
        .await?;
        let prepared = self
            .prepare_chapter_generation_application(PrepareChapterGenerationInput {
                project_id: input.project_id,
                chapter_number: input.chapter_number,
                writing_notes: input.writing_notes.as_deref(),
                min_word_count: input.min_word_count,
                max_word_count: input.max_word_count,
                client_rag_context: None,
                runtime_embedding_identity: context.runtime_embedding_identity,
                cancel_token: None,
            })
            .await?;
        let system_prompt = context
            .prompt_renderer
            .render_prompt("writing", &prepared.variables, Some(input.project_id))
            .await
            .map_err(|error| format!("渲染 writing 提示词失败: {error}"))?;
        let request = a4n_prompt::chapter_generation::build_chapter_generation_request(
            input.project_id,
            input.chapter_number,
            system_prompt,
            &prepared.outline_text,
            prepared.context_before.as_deref(),
        )
        .map_err(|error| format!("构建章节生成请求失败: {error}"))?;
        Ok(ChapterGenerationRequestPreview {
            title: format!("第 {} 章生成请求预览", input.chapter_number),
            note: format!(
                "章节：{}；字数：{}-{}；检索参考：{}",
                prepared.chapter_title,
                prepared.word_count_range.min,
                prepared.word_count_range.max,
                if prepared.use_rag {
                    "启用"
                } else {
                    "未启用"
                }
            ),
            request,
        })
    }

    pub async fn prepare_chapter_generation_rag_embedding_validated(
        &self,
        user_id: i64,
        input: PreviewChapterGenerationInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
        self.validate_chapter_generation_application_access(
            user_id,
            input.project_id,
            input.chapter_number,
            "准备章节生成 RAG 检索",
        )
        .await?;
        let embedding_identity = self
            .effective_chapter_generation_embedding_identity(runtime_embedding_identity)
            .await?;
        if input.chapter_number <= 1 {
            return Ok(PreparedGenerationRagEmbeddingRequest {
                query_texts: Vec::new(),
                embedding_identity,
            });
        }
        let (outline, writing_instruction, rag_query_outline_text) = self
            .chapter_generation_rag_input(
                input.project_id,
                input.chapter_number,
                input.writing_notes.as_deref(),
            )
            .await?;
        let prepared = self
            .prepare_generation_rag_search(
                input.project_id,
                input.chapter_number,
                Some(&outline),
                &rag_query_outline_text,
                writing_instruction.as_deref(),
                embedding_identity.clone(),
            )
            .await?;
        Ok(PreparedGenerationRagEmbeddingRequest {
            query_texts: prepared
                .queries
                .weighted_queries()
                .into_iter()
                .map(|query| query.text.to_string())
                .collect(),
            embedding_identity,
        })
    }

    pub async fn finish_chapter_generation_rag_context_validated(
        &self,
        user_id: i64,
        input: FinishChapterGenerationRagContextInput,
    ) -> Result<GenerationRagContext, String> {
        self.validate_chapter_generation_application_access(
            user_id,
            input.project_id,
            input.chapter_number,
            "构建章节生成 RAG 上下文",
        )
        .await?;
        if input.chapter_number <= 1 {
            return Ok(GenerationRagContext::default());
        }
        let (outline, writing_instruction, rag_query_outline_text) = self
            .chapter_generation_rag_input(
                input.project_id,
                input.chapter_number,
                input.writing_notes.as_deref(),
            )
            .await?;
        let prepared = self
            .prepare_generation_rag_search(
                input.project_id,
                input.chapter_number,
                Some(&outline),
                &rag_query_outline_text,
                writing_instruction.as_deref(),
                input.embedding_identity.clone(),
            )
            .await?;
        self.execute_prepared_chapter_generation_rag_search_with_vectors(
            &prepared,
            &input.embedding_identity,
            input.query_vectors,
        )
        .await
    }

    pub(crate) async fn validate_chapter_generation_application_access(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        operation: &str,
    ) -> Result<(), String> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await
            .map_err(|error| error.to_string())?;
        self.ensure_text_source_project(project_id, operation)
            .await?;
        self.validate_chapter_generation_readiness(project_id, chapter_number)
            .await?;
        self.preflight_chapter_content_change_validated(project_id, chapter_number)
            .await
            .map_err(|error| error.to_string())
    }

    async fn prepare_chapter_generation_application(
        &self,
        input: PrepareChapterGenerationInput<'_>,
    ) -> Result<PreparedChapterGeneration, String> {
        let outline = self
            .required_chapter_generation_outline(input.project_id, input.chapter_number)
            .await?;
        let writing_instruction = input.writing_notes.and_then(non_empty_trimmed);
        let word_count_range = resolve_generation_word_count_range(
            Some(&outline),
            input.min_word_count,
            input.max_word_count,
        )?;
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
        let mut outline_text = merge_generation_outline_text(
            input.chapter_number,
            Some(&outline),
            None,
            writing_instruction,
            Some(&chapter_title),
            Some(word_count_range),
        )?;

        check_optional_cancelled(input.cancel_token)?;
        let reference_context = self
            .resolve_generation_reference_context(
                input.project_id,
                input.chapter_number,
                Some(&outline),
            )
            .await?;

        check_optional_cancelled(input.cancel_token)?;
        let rag_context = self
            .resolve_chapter_generation_rag_context(
                input.project_id,
                input.chapter_number,
                &outline,
                &rag_query_outline_text,
                writing_instruction,
                input.client_rag_context,
                input.runtime_embedding_identity,
            )
            .await?;

        check_optional_cancelled(input.cancel_token)?;
        let context_before = self
            .resolve_chapter_context_before(input.project_id, input.chapter_number, None)
            .await
            .map_err(|error| format!("构建章节前文上下文失败: {error}"))?;

        for context in [reference_context.as_deref(), rag_context.as_deref()]
            .into_iter()
            .flatten()
        {
            outline_text.push_str("\n\n");
            outline_text.push_str(context);
        }
        let mut variables = HashMap::new();
        upsert_chapter_generation_variables(
            &mut variables,
            input.chapter_number,
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
            use_rag: input.chapter_number > 1,
            variables,
        })
    }

    async fn resolve_chapter_generation_rag_context(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: &chapter_outline::Model,
        rag_query_outline_text: &str,
        writing_instruction: Option<&str>,
        client_rag_context: Option<&str>,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<Option<String>, String> {
        if chapter_number <= 1 {
            return Ok(None);
        }
        if let Some(context) = client_rag_context
            .map(str::trim)
            .filter(|context| !context.is_empty())
        {
            return Ok(Some(context.to_string()));
        }
        let embedding_identity = self
            .effective_chapter_generation_embedding_identity(runtime_embedding_identity)
            .await?;
        let prepared = self
            .prepare_generation_rag_search(
                project_id,
                chapter_number,
                Some(outline),
                rag_query_outline_text,
                writing_instruction,
                embedding_identity,
            )
            .await?;
        self.execute_prepared_chapter_generation_rag_search(&prepared)
            .await
            .map(|context| context.text)
    }

    async fn prepare_generation_rag_search(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: Option<&chapter_outline::Model>,
        outline_text: &str,
        writing_instruction: Option<&str>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<PreparedGenerationRagSearch, String> {
        let queries = self
            .prepare_chapter_generation_rag_queries(
                project_id,
                chapter_number,
                outline,
                outline_text,
                writing_instruction,
            )
            .await?;
        self.prepare_chapter_generation_rag_search(
            project_id,
            chapter_number,
            queries,
            embedding_identity,
        )
        .await
    }

    async fn chapter_generation_rag_input(
        &self,
        project_id: i64,
        chapter_number: i32,
        writing_notes: Option<&str>,
    ) -> Result<(chapter_outline::Model, Option<String>, String), String> {
        let outline = self
            .required_chapter_generation_outline(project_id, chapter_number)
            .await?;
        let writing_instruction = writing_notes.and_then(non_empty_trimmed);
        let chapter_title =
            required_generation_chapter_title(chapter_number, Some(&outline), None)?;
        let rag_query_outline_text = merge_generation_outline_text(
            chapter_number,
            Some(&outline),
            None,
            writing_instruction,
            Some(&chapter_title),
            None,
        )?;
        Ok((
            outline,
            writing_instruction.map(str::to_string),
            rag_query_outline_text,
        ))
    }

    async fn effective_chapter_generation_embedding_identity(
        &self,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<EmbeddingModelIdentity, String> {
        match runtime_embedding_identity {
            Some(identity) => Ok(identity),
            None => self
                .embedding_model_identity()
                .await
                .map_err(|error| error.to_string()),
        }
    }
}

async fn fail_chapter_generation_execution(
    orchestrator: &OrchestratorContext,
    chapter_service: &ChapterGenerationService,
    execution: GenerationTaskExecution,
    chapter_session: Option<Arc<ChapterGenerationSession>>,
    workflow_run_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    if let Some(chapter_session) = chapter_session.as_deref() {
        chapter_service
            .restore_generation_status_ref(chapter_session)
            .await;
    }
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        orchestrator
            .fail_chapter_generation_workflow(workflow_run_id, &error.message)
            .await
            .err()
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some(workflow_error) = workflow_error {
        message.push_str(&format!(
            "; 记录章节生成工作流失败状态失败: {workflow_error}"
        ));
    }
    Err(message)
}

fn chapter_progress(
    stage_code: &str,
    phase_kind: ProgressPhaseKind,
    message: &str,
) -> ProgressEvent {
    ProgressEvent {
        stage_code: stage_code.to_string(),
        phase_kind,
        message: message.to_string(),
        current: None,
        total: None,
        unit_label: None,
        item_key: None,
        item_index: None,
        item_total: None,
        severity: ProgressSeverity::Info,
        estimated: false,
    }
}

fn generation_error_from_app(code: &str, stage_code: &str, error: AppError) -> GenerationError {
    generation_error_from_text(code, stage_code, error.to_string())
}

fn generation_error_from_session(error: GenerationSessionError) -> GenerationError {
    generation_error_from_text(error.code(), "checkpoint", error.to_string())
}

fn generation_error_from_terminal(error: GenerationTerminalError) -> GenerationError {
    generation_error_from_text(error.code(), "persist", error.to_string())
}

fn generation_error_from_text(code: &str, stage_code: &str, message: String) -> GenerationError {
    generation_public_error(code, stage_code, message)
}

fn generation_finish_reason_text(reason: GenerationFinishReason) -> String {
    shared_generation_finish_reason_text(reason)
}

fn check_optional_cancelled(cancel_token: Option<&CancellationToken>) -> Result<(), String> {
    if let Some(cancel_token) = cancel_token {
        check_cancelled(cancel_token)?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    cancel_token
        .is_cancelled()
        .then(|| AppError::Cancelled.to_string())
        .map_or(Ok(()), Err)
}
