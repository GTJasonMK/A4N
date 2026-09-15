use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use a4n_api_contract::{
    chapter::{
        ChapterDerivedCleanupHostPort, PreparedGenerationRagEmbeddingRequest,
        PreviewChapterPromptInput, StartChapterGenerationInput,
    },
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
};
use a4n_db::workflow;
use a4n_db_models::models::chapter::chapter_outline;
use a4n_domain::progress::ProgressPhaseKind;
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_service_protocol::{GenerationError, GenerationResultRef, StreamTextHash};
use futures::future::BoxFuture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    context::{GenerationTaskExecution, fail_generation_workflow_execution},
    generation::{
        checkpoint::GenerationCheckpointMetadata,
        events::GenerationEventPublisher,
        generation_finish_reason_text as shared_generation_finish_reason_text,
        generation_progress as chapter_progress, generation_public_error, generation_session_error,
        generation_terminal_error,
        stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
        terminal::CompletionDescriptor,
    },
    workflow::workflow_checkpoint_contract_app_error,
};

use super::service::{
    ChapterGenerationService, ChapterGenerationSession,
    generation::checkpoint::{
        ChapterGenerationCheckpointSaveInput, chapter_generation_call_fingerprint_json,
        save_completed_chapter_generation_checkpoint,
        validate_chapter_generation_source_fingerprint,
    },
};

use super::{
    ChapterGenerationApplicationContext, ChapterGenerationApplicationPort,
    ChapterGenerationRagPreparationApplicationPort, ChapterGenerationRequestPreview,
    ChapterGenerationRequestPreviewApplicationPort, ChapterGenerationWordCountRange,
    generation_chapter_title, generation_rag_query_fingerprint, merge_generation_outline_text,
    non_empty_trimmed, resolve_generation_word_count_range, upsert_chapter_generation_variables,
    validate_generation_rag_query_fingerprint,
};
use a4n_rag_core::validation::validate_query_vectors;

struct PreparedChapterGeneration {
    chapter_title: String,
    source_outline: Option<chapter_outline::Model>,
    outline_text: String,
    context_before: Option<String>,
    word_count_range: ChapterGenerationWordCountRange,
    use_rag: bool,
    variables: HashMap<String, Value>,
    source_fingerprint_json: Value,
    rag_embedding_identity: Option<EmbeddingModelIdentity>,
}

struct PreparedChapterGenerationSources {
    chapter_title: String,
    source_outline: Option<chapter_outline::Model>,
    writing_instruction: Option<String>,
    rag_query_outline_text: String,
    base_outline_text: String,
    reference_context: Option<String>,
    context_before: Option<String>,
    word_count_range: ChapterGenerationWordCountRange,
}

struct ResolvedChapterGenerationRagContext {
    text: Option<String>,
    embedding_identity: Option<EmbeddingModelIdentity>,
    query_fingerprint: Option<String>,
}

#[derive(Debug)]
struct ClientGenerationRagInput {
    embedding_identity: EmbeddingModelIdentity,
    query_fingerprint: String,
    query_vectors: Vec<Vec<f32>>,
}

struct PrepareChapterGenerationInput<'a> {
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<&'a str>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag: Option<ClientGenerationRagInput>,
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
        host_port: Arc<dyn ChapterDerivedCleanupHostPort>,
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
        let publisher = execution.publisher.clone();
        let chapter_service = self.chapter_generation_service();
        let client_rag = match client_generation_rag_input(&input) {
            Ok(client_rag) => client_rag,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    None,
                    None,
                    generation_public_error("generation_request_invalid", "chapter_prepare", error),
                )
                .await;
            }
        };

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
                generation_session_error(error),
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
                    generation_public_error(
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
                client_rag,
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
                    generation_public_error("generation_request_invalid", "chapter_prepare", error),
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
                generation_session_error(error),
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
                    generation_public_error("generation_request_invalid", "chapter_prompt", error),
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
                    generation_public_error(
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
                    generation_public_error("generation_persist_failed", "chapter_workflow", error),
                )
                .await;
            }
        };
        let call_key = format!("chapter_generation:chapter:{}", input.chapter_number);
        let fingerprint = || -> Result<_, AppError> {
            let json = chapter_generation_call_fingerprint_json(
                input.project_id,
                input.chapter_number,
                &prepared.source_fingerprint_json,
                &request,
            )?;
            let sha256 = workflow::llm_call_checkpoint_fingerprint_sha256(&json)
                .map_err(workflow_checkpoint_contract_app_error)?;
            Ok((json, sha256))
        };
        let (input_fingerprint_json, input_fingerprint_sha256) = match fingerprint() {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    Some(workflow_run_id),
                    generation_public_error("generation_request_invalid", "chapter_prompt", error),
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
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let checkpoint_metadata = GenerationCheckpointMetadata {
            step_key: Some("chapter_generation".to_string()),
            call_key: Some(call_key.clone()),
            provider_model: Some(model_name.clone()),
            ..Default::default()
        };

        if let Err(error) = publisher
            .progress(chapter_progress(
                "chapter_stream",
                ProgressPhaseKind::ModelStream,
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
                generation_session_error(error),
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
                generation_public_error(
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
                    generation_public_error(
                        "generation_validation_failed",
                        "chapter_validate",
                        error,
                    ),
                )
                .await;
            }
        };
        let current_source_fingerprint = match self
            .current_chapter_generation_source_fingerprint(
                input.project_id,
                input.chapter_number,
                input.writing_notes.as_deref(),
                input.min_word_count,
                input.max_word_count,
                prepared.rag_embedding_identity.as_ref(),
                Some(&cancel_token),
            )
            .await
        {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                return fail_chapter_generation_execution(
                    self,
                    &chapter_service,
                    execution,
                    Some(Arc::clone(&chapter_session)),
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_source_revalidation_failed",
                        "chapter_source_revalidate",
                        error,
                    ),
                )
                .await;
            }
        };
        if let Err(error) = validate_chapter_generation_source_fingerprint(
            &prepared.source_fingerprint_json,
            &current_source_fingerprint,
        ) {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                Some(Arc::clone(&chapter_session)),
                Some(workflow_run_id),
                generation_public_error(
                    "generation_source_changed",
                    "chapter_source_revalidate",
                    error,
                ),
            )
            .await;
        }
        let finish_reason_text = shared_generation_finish_reason_text(output.finish_reason);
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
                generation_public_error("generation_persist_failed", "chapter_checkpoint", error),
            )
            .await;
        }

        let content_sync_lock =
            self.chapter_content_sync_lock(input.project_id, input.chapter_number);
        let _content_sync_guard = content_sync_lock.lock_owned().await;
        let chapter_id = Arc::new(AtomicI64::new(0));
        let chapter_id_slot = Arc::clone(&chapter_id);
        let content_sync_run_id = Arc::new(AtomicI64::new(0));
        let content_sync_run_id_slot = Arc::clone(&content_sync_run_id);
        let finish_reason = output.finish_reason;
        let content = output.content;
        let source_outline = prepared.source_outline;
        let outline_text = prepared.outline_text;
        let trace = crate::chapter_generation::service::ChapterGenerationTrace {
            request: request_trace,
            finish_reason: Some(finish_reason_text.clone()),
            model_name,
        };
        let completion_service = chapter_service.clone();
        let completion_chapter_session = Arc::clone(&chapter_session);
        let completion_context = self.clone();
        let completion_host_port = Arc::clone(&host_port);
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let chapter_service = completion_service.clone();
                let chapter_session = Arc::clone(&completion_chapter_session);
                let content = content.clone();
                let source_outline = source_outline;
                let outline_text = outline_text.clone();
                let trace = trace.clone();
                let context = completion_context.clone();
                Box::pin(async move {
                    let chapter = chapter_service
                        .finish_generated_chapter_in_transaction(
                            &chapter_session,
                            &content,
                            source_outline,
                            &outline_text,
                            Some(trace),
                            tx,
                        )
                        .await?;
                    let content_sync_run = context
                        .begin_chapter_content_sync_in_transaction(tx, &chapter, &|artifacts| {
                            completion_host_port.validate_chapter_cleanup_artifacts(artifacts)
                        })
                        .await?;
                    chapter_id_slot.store(chapter.id, Ordering::Release);
                    content_sync_run_id_slot.store(content_sync_run.id, Ordering::Release);
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
                generation_terminal_error(error),
            )
            .await;
        }

        execution.finish();
        let chapter_id = chapter_id.load(Ordering::Acquire);
        if chapter_id <= 0 {
            return Err("generation completed 但缺少章节结果 id".to_string());
        }
        let content_sync_run_id = content_sync_run_id.load(Ordering::Acquire);
        if content_sync_run_id <= 0 {
            return Err("generation completed 但缺少章节资料同步 run id".to_string());
        }
        let content_sync_run = self
            .workflow_run_model_by_id(content_sync_run_id)
            .await
            .map_err(|error| error.to_string())?;
        if let Err(error) = self
            .settle_chapter_content_sync_run(host_port.as_ref(), &content_sync_run)
            .await
        {
            tracing::warn!(
                task_id = %handle.task_id,
                chapter_id,
                workflow_run_id = content_sync_run_id,
                error = %error,
                "章节正文已完成，派生资料同步待恢复"
            );
        }
        self.complete_workflow_run_validated(workflow_run_id)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn preview_chapter_generation_request_validated(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterPromptInput,
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
                client_rag: None,
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
        input: PreviewChapterPromptInput,
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
            let query_fingerprint = generation_rag_query_fingerprint(
                input.project_id,
                input.chapter_number,
                &[],
                &embedding_identity,
            )?;
            return Ok(PreparedGenerationRagEmbeddingRequest {
                query_texts: Vec::new(),
                embedding_identity: a4n_api_contract::embedding::EmbeddingModelIdentity {
                    backend: embedding_identity.backend,
                    model: embedding_identity.model,
                    dimension: embedding_identity.dimension,
                },
                query_fingerprint,
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
                outline.as_ref(),
                &rag_query_outline_text,
                writing_instruction.as_deref(),
                embedding_identity,
            )
            .await?;
        let query_fingerprint = generation_rag_query_fingerprint(
            prepared.project_id,
            prepared.chapter_number,
            &prepared.queries,
            &prepared.embedding_identity,
        )?;
        let query_texts = prepared
            .queries
            .into_iter()
            .map(|query| query.text)
            .collect();
        let embedding_identity = prepared.embedding_identity;
        Ok(PreparedGenerationRagEmbeddingRequest {
            query_fingerprint,
            query_texts,
            embedding_identity: a4n_api_contract::embedding::EmbeddingModelIdentity {
                backend: embedding_identity.backend,
                model: embedding_identity.model,
                dimension: embedding_identity.dimension,
            },
        })
    }

    pub(crate) async fn validate_chapter_generation_client_rag_input(
        &self,
        input: &StartChapterGenerationInput,
    ) -> Result<(), String> {
        let Some(client_rag) = client_generation_rag_input(input)? else {
            return Ok(());
        };
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
                outline.as_ref(),
                &rag_query_outline_text,
                writing_instruction.as_deref(),
                client_rag.embedding_identity,
            )
            .await?;
        validate_generation_rag_query_fingerprint(&client_rag.query_fingerprint, &prepared)?;
        validate_query_vectors(
            &client_rag.query_vectors,
            prepared.queries.len(),
            prepared.embedding_identity.dimension,
        )
        .map_err(|error| error.to_string())
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
        self.validate_chapter_generation_readiness(project_id, chapter_number)?;
        self.preflight_chapter_content_change_validated(project_id, chapter_number)
            .await
            .map_err(|error| error.to_string())
    }

    async fn prepare_chapter_generation_application(
        &self,
        input: PrepareChapterGenerationInput<'_>,
    ) -> Result<PreparedChapterGeneration, String> {
        let sources = self
            .prepare_chapter_generation_sources(
                input.project_id,
                input.chapter_number,
                input.writing_notes,
                input.min_word_count,
                input.max_word_count,
                input.cancel_token,
            )
            .await?;

        check_optional_cancelled(input.cancel_token)?;
        let rag_context = self
            .resolve_chapter_generation_rag_context(
                input.project_id,
                input.chapter_number,
                sources.source_outline.as_ref(),
                &sources.rag_query_outline_text,
                sources.writing_instruction.as_deref(),
                input.client_rag,
                input.runtime_embedding_identity,
            )
            .await?;
        let source_fingerprint_json = self
            .chapter_generation_source_fingerprint_json(
                input.project_id,
                input.chapter_number,
                &sources,
                rag_context.query_fingerprint.as_deref(),
            )
            .await?;

        let mut outline_text = sources.base_outline_text.clone();
        for context in [
            sources.reference_context.as_deref(),
            rag_context.text.as_deref(),
        ]
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
            &sources.chapter_title,
            &outline_text,
            sources.writing_instruction.as_deref(),
            sources.reference_context.as_deref(),
            rag_context.text.as_deref(),
            sources.word_count_range,
        );
        Ok(PreparedChapterGeneration {
            chapter_title: sources.chapter_title,
            source_outline: sources.source_outline,
            outline_text,
            context_before: sources.context_before,
            word_count_range: sources.word_count_range,
            use_rag: input.chapter_number > 1,
            variables,
            source_fingerprint_json,
            rag_embedding_identity: rag_context.embedding_identity,
        })
    }

    async fn prepare_chapter_generation_sources(
        &self,
        project_id: i64,
        chapter_number: i32,
        writing_notes: Option<&str>,
        min_word_count: Option<i32>,
        max_word_count: Option<i32>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<PreparedChapterGenerationSources, String> {
        let outline = self
            .formal_chapter_outline_by_project_number(project_id, chapter_number)
            .await
            .map_err(|error| error.to_string())?;
        let writing_instruction = writing_notes
            .and_then(non_empty_trimmed)
            .map(str::to_string);
        let word_count_range =
            resolve_generation_word_count_range(outline.as_ref(), min_word_count, max_word_count)?;
        let chapter_title = generation_chapter_title(chapter_number, outline.as_ref(), None);
        let rag_query_outline_text = merge_generation_outline_text(
            chapter_number,
            outline.as_ref(),
            None,
            writing_instruction.as_deref(),
            Some(&chapter_title),
            None,
        )?;
        let base_outline_text = merge_generation_outline_text(
            chapter_number,
            outline.as_ref(),
            None,
            writing_instruction.as_deref(),
            Some(&chapter_title),
            Some(word_count_range),
        )?;

        check_optional_cancelled(cancel_token)?;
        let (reference_result, context_before_result) = futures::join!(
            self.resolve_generation_reference_context(project_id, chapter_number, outline.as_ref()),
            self.resolve_chapter_context_before(project_id, chapter_number, None),
        );
        let reference_context = reference_result?;
        let context_before =
            context_before_result.map_err(|error| format!("构建章节前文上下文失败: {error}"))?;

        Ok(PreparedChapterGenerationSources {
            chapter_title,
            source_outline: outline,
            writing_instruction,
            rag_query_outline_text,
            base_outline_text,
            reference_context,
            context_before,
            word_count_range,
        })
    }

    async fn resolve_chapter_generation_rag_context(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: Option<&chapter_outline::Model>,
        rag_query_outline_text: &str,
        writing_instruction: Option<&str>,
        client_rag: Option<ClientGenerationRagInput>,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<ResolvedChapterGenerationRagContext, String> {
        if chapter_number <= 1 {
            return Ok(ResolvedChapterGenerationRagContext {
                text: None,
                embedding_identity: None,
                query_fingerprint: None,
            });
        }
        let embedding_identity = match client_rag.as_ref() {
            Some(client_rag) => client_rag.embedding_identity.clone(),
            None => {
                self.effective_chapter_generation_embedding_identity(runtime_embedding_identity)
                    .await?
            }
        };
        let prepared = self
            .prepare_generation_rag_search(
                project_id,
                chapter_number,
                outline,
                rag_query_outline_text,
                writing_instruction,
                embedding_identity,
            )
            .await?;
        let query_fingerprint = generation_rag_query_fingerprint(
            prepared.project_id,
            prepared.chapter_number,
            &prepared.queries,
            &prepared.embedding_identity,
        )?;
        let context = match client_rag {
            Some(client_rag) => {
                validate_generation_rag_query_fingerprint(
                    &client_rag.query_fingerprint,
                    &prepared,
                )?;
                self.execute_prepared_chapter_generation_rag_search_with_vectors(
                    &prepared,
                    &client_rag.embedding_identity,
                    client_rag.query_vectors,
                )
                .await?
            }
            None => {
                self.execute_prepared_chapter_generation_rag_search(&prepared)
                    .await?
            }
        };
        let embedding_identity = prepared.embedding_identity;
        Ok(ResolvedChapterGenerationRagContext {
            text: context,
            embedding_identity: Some(embedding_identity),
            query_fingerprint: Some(query_fingerprint),
        })
    }

    async fn current_chapter_generation_source_fingerprint(
        &self,
        project_id: i64,
        chapter_number: i32,
        writing_notes: Option<&str>,
        min_word_count: Option<i32>,
        max_word_count: Option<i32>,
        rag_embedding_identity: Option<&EmbeddingModelIdentity>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<Value, String> {
        let sources = self
            .prepare_chapter_generation_sources(
                project_id,
                chapter_number,
                writing_notes,
                min_word_count,
                max_word_count,
                cancel_token,
            )
            .await?;
        let rag_query_fingerprint = if chapter_number <= 1 {
            None
        } else {
            let embedding_identity = rag_embedding_identity.ok_or_else(|| {
                "第 2 章及以后缺少章节生成 RAG embedding identity，无法复验正式来源".to_string()
            })?;
            let prepared = self
                .prepare_generation_rag_search(
                    project_id,
                    chapter_number,
                    sources.source_outline.as_ref(),
                    &sources.rag_query_outline_text,
                    sources.writing_instruction.as_deref(),
                    embedding_identity.clone(),
                )
                .await?;
            Some(generation_rag_query_fingerprint(
                prepared.project_id,
                prepared.chapter_number,
                &prepared.queries,
                &prepared.embedding_identity,
            )?)
        };
        check_optional_cancelled(cancel_token)?;
        self.chapter_generation_source_fingerprint_json(
            project_id,
            chapter_number,
            &sources,
            rag_query_fingerprint.as_deref(),
        )
        .await
    }

    async fn chapter_generation_source_fingerprint_json(
        &self,
        project_id: i64,
        chapter_number: i32,
        sources: &PreparedChapterGenerationSources,
        rag_query_fingerprint: Option<&str>,
    ) -> Result<Value, String> {
        let rag_source_revision_sha256 = self
            .chapter_generation_rag_source_revision_sha256(project_id, chapter_number)
            .await?;
        Ok(json!({
            "version": 1,
            "project_id": project_id,
            "chapter_number": chapter_number,
            "chapter_title": sources.chapter_title,
            "base_outline_text": sources.base_outline_text,
            "reference_context": sources.reference_context,
            "context_before": sources.context_before,
            "word_count_range": sources.word_count_range,
            "rag_query_fingerprint": rag_query_fingerprint,
            "rag_source_revision_sha256": rag_source_revision_sha256,
        }))
    }

    async fn chapter_generation_rag_source_revision_sha256(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<String>, String> {
        if chapter_number <= 1 {
            return Ok(None);
        }
        let (chapters_result, profiles_result) = futures::join!(
            self.chapters_before_number(project_id, chapter_number),
            self.protagonist_profiles_for_project(project_id),
        );
        let chapters = chapters_result
            .map_err(|error| format!("读取章节生成前序正文来源失败: {error}"))?
            .into_iter()
            .filter(|chapter| chapter.has_available_content())
            .collect::<Vec<_>>();
        let profiles =
            profiles_result.map_err(|error| format!("读取章节生成角色来源失败: {error}"))?;
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let (character_states_result, attribute_events_result) = futures::join!(
            self.character_states_for_rag_ingest_before_chapter(&profile_ids, chapter_number),
            self.attribute_events_for_rag_ingest_before_chapter(&profile_ids, chapter_number),
        );
        let character_states = character_states_result
            .map_err(|error| format!("读取章节生成角色状态来源失败: {error}"))?;
        let attribute_events = attribute_events_result
            .map_err(|error| format!("读取章节生成角色属性来源失败: {error}"))?;

        let sources = json!({
            "chapters": chapters,
            "character_profiles": profiles,
            "character_states": character_states,
            "character_attribute_events": attribute_events,
        });
        workflow::llm_call_checkpoint_fingerprint_sha256(&sources)
            .map(Some)
            .map_err(|error| format!("构建章节生成正式 RAG 来源 fingerprint 失败: {error}"))
    }

    pub(crate) async fn validate_chapter_generation_plan_input(
        &self,
        project_id: i64,
        chapter_number: i32,
        writing_notes: Option<&str>,
    ) -> Result<(), String> {
        self.chapter_generation_rag_input(project_id, chapter_number, writing_notes)
            .await
            .map(|_| ())
    }

    async fn chapter_generation_rag_input(
        &self,
        project_id: i64,
        chapter_number: i32,
        writing_notes: Option<&str>,
    ) -> Result<(Option<chapter_outline::Model>, Option<String>, String), String> {
        let outline = self
            .formal_chapter_outline_by_project_number(project_id, chapter_number)
            .await
            .map_err(|error| error.to_string())?;
        let writing_instruction = writing_notes.and_then(non_empty_trimmed);
        let chapter_title = generation_chapter_title(chapter_number, outline.as_ref(), None);
        let rag_query_outline_text = merge_generation_outline_text(
            chapter_number,
            outline.as_ref(),
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

#[async_trait::async_trait]
impl ChapterGenerationRequestPreviewApplicationPort for OrchestratorContext {
    async fn preview_request(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterPromptInput,
    ) -> Result<ChapterGenerationRequestPreview, String> {
        OrchestratorContext::preview_chapter_generation_request_validated(
            self, user_id, context, input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterGenerationRagPreparationApplicationPort for OrchestratorContext {
    async fn prepare_rag_embedding(
        &self,
        user_id: i64,
        input: PreviewChapterPromptInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
        OrchestratorContext::prepare_chapter_generation_rag_embedding_validated(
            self,
            user_id,
            input,
            runtime_embedding_identity,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterGenerationApplicationPort for OrchestratorContext {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<crate::GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_chapter_generation_task(self, user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: StartChapterGenerationInput,
        handle: GenerationTaskHandle,
        host_port: Arc<dyn ChapterDerivedCleanupHostPort>,
    ) -> Result<(), String> {
        OrchestratorContext::run_chapter_generation_task(self, context, input, handle, host_port)
            .await
    }
}

fn client_generation_rag_input(
    input: &StartChapterGenerationInput,
) -> Result<Option<ClientGenerationRagInput>, String> {
    let client_rag = match (
        input.client_rag_embedding_identity.as_ref(),
        input.client_rag_query_fingerprint.as_deref(),
        input.client_rag_query_vectors.as_ref(),
    ) {
        (None, None, None) => return Ok(None),
        (Some(identity), Some(query_fingerprint), Some(query_vectors)) => {
            ClientGenerationRagInput {
                embedding_identity: EmbeddingModelIdentity {
                    backend: identity.backend.clone(),
                    model: identity.model.clone(),
                    dimension: identity.dimension,
                },
                query_fingerprint: query_fingerprint.to_string(),
                query_vectors: query_vectors.clone(),
            }
        }
        _ => {
            return Err(
                "章节生成客户端 RAG identity、query fingerprint 和 query vectors 必须同时提供"
                    .to_string(),
            );
        }
    };
    if input.chapter_number <= 1 {
        return Err("第 1 章不接受客户端 RAG 查询向量".to_string());
    }
    Ok(Some(client_rag))
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
    fail_generation_workflow_execution(orchestrator, execution, workflow_run_id, error).await
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

#[cfg(test)]
mod cancellation_tests {
    use super::{StartChapterGenerationInput, client_generation_rag_input};

    #[test]
    fn chapter_generation_start_errors_use_stable_codes_and_retry_policy() {
        let invalid = crate::GenerationTaskAccessError::InvalidArgument("请求无效".to_string())
            .into_start_error();
        assert_eq!(invalid.code(), "generation_request_invalid");
        assert!(!invalid.retryable);

        let forbidden =
            crate::GenerationTaskAccessError::Forbidden("无权限".to_string()).into_start_error();
        assert_eq!(forbidden.code(), "generation_task_forbidden");
        assert!(!forbidden.retryable);
    }

    fn chapter_generation_input(chapter_number: i32) -> StartChapterGenerationInput {
        StartChapterGenerationInput {
            meta: a4n_api_contract::generation_tasks::GenerationStartRequestMeta {
                start_request_id: "test-request".to_string(),
                retry_of_task_id: None,
            },
            project_id: 1,
            chapter_number,
            writing_notes: None,
            min_word_count: None,
            max_word_count: None,
            client_rag_embedding_identity: None,
            client_rag_query_fingerprint: None,
            client_rag_query_vectors: None,
        }
    }

    #[test]
    fn client_rag_fields_must_be_provided_together() {
        let mut input = chapter_generation_input(2);
        input.client_rag_query_fingerprint = Some("a".repeat(64));

        assert!(
            client_generation_rag_input(&input)
                .expect_err("partial client RAG payload must fail")
                .contains("必须同时提供")
        );
    }

    #[test]
    fn first_chapter_rejects_client_rag_vectors() {
        let mut input = chapter_generation_input(1);
        input.client_rag_embedding_identity =
            Some(a4n_api_contract::embedding::EmbeddingModelIdentity {
                backend: "local".to_string(),
                model: "test".to_string(),
                dimension: 2,
            });
        input.client_rag_query_fingerprint = Some("a".repeat(64));
        input.client_rag_query_vectors = Some(vec![vec![0.0, 1.0]]);

        assert!(
            client_generation_rag_input(&input)
                .expect_err("chapter one must not accept client RAG vectors")
                .contains("第 1 章")
        );
    }
}
