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

        if let Err(error) = self
            .ensure_prior_chapter_context_ready(input.project_id, input.chapter_number)
            .await
        {
            return fail_chapter_generation_execution(
                self,
                &chapter_service,
                execution,
                None,
                None,
                generation_public_error("generation_validation_failed", "chapter_prepare", error),
            )
            .await;
        }

        let chapter_session = match chapter_service
            .begin_generation(input.project_id, input.chapter_number, &handle.task_id)
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
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let chapter_service = completion_service.clone();
                let chapter_session = Arc::clone(&completion_chapter_session);
                let content = content.clone();
                let source_outline = source_outline;
                let outline_text = outline_text.clone();
                let trace = trace.clone();
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
                    let content_sync_run =
                        a4n_chapter_content_core::begin_chapter_content_sync_in_transaction(
                            tx, &chapter,
                        )
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
            .workflow_run_summary(content_sync_run_id)
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
        self.ensure_prior_chapter_context_ready(project_id, chapter_number)
            .await
    }

    async fn ensure_prior_chapter_context_ready(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), String> {
        let chapters = self
            .chapters_before_number(project_id, chapter_number)
            .await
            .map_err(|error| format!("读取前文章节资料状态失败: {error}"))?;
        let chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<std::collections::HashSet<_>>();
        if chapter_ids.is_empty() {
            return Ok(());
        }
        let runs = self
            .workflow_runs_by_project_kind(project_id, "chapter_analysis")
            .await
            .map_err(|error| format!("读取前文章节分析状态失败: {error}"))?;
        if let Some(run) = runs.into_iter().find(|run| {
            run.chapter_id
                .is_some_and(|chapter_id| chapter_ids.contains(&chapter_id))
                && (run.status == crate::workflow::WORKFLOW_STATUS_RUNNING
                    || (run.can_resume
                        && matches!(
                            run.status.as_str(),
                            crate::workflow::WORKFLOW_STATUS_INTERRUPTED
                                | crate::workflow::WORKFLOW_STATUS_FAILED
                                | crate::workflow::WORKFLOW_STATUS_CANCELLED
                        )))
        }) {
            return Err(format!(
                "第 {} 章之前仍有未完成的章节资料重放（workflow_run_id={}），请先恢复同步",
                chapters
                    .iter()
                    .find(|chapter| Some(chapter.id) == run.chapter_id)
                    .map(|chapter| chapter.chapter_number)
                    .unwrap_or_default(),
                run.id
            ));
        }
        for chapter in &chapters {
            validate_prior_chapter_context(
                chapter.chapter_number,
                chapter.is_generating(),
                &chapter.content,
                chapter.analysis_data.as_ref(),
            )?;
        }
        Ok(())
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
        self.ensure_prior_chapter_context_ready(input.project_id, input.chapter_number)
            .await
            .map_err(|error| format!("准备生成请求期间前文状态发生变化：{error}"))?;

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
        outline: Option<&ChapterOutlineRecord>,
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
    ) -> Result<(Option<ChapterOutlineRecord>, Option<String>, String), String> {
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

fn validate_prior_chapter_context(
    chapter_number: i32,
    is_generating: bool,
    content: &str,
    analysis_data: Option<&Value>,
) -> Result<(), String> {
    if is_generating {
        return Err(format!(
            "第 {chapter_number} 章正文仍在生成中，请等待完成后再生成后续章节"
        ));
    }
    if content.trim().is_empty() {
        return Ok(());
    }
    let analysis = analysis_data.ok_or_else(|| {
        format!("第 {chapter_number} 章缺少有效章节分析，请先完成该章分析后再生成后续章节")
    })?;
    a4n_chapter_analysis::validate_saved_chapter_analysis(analysis).map_err(|error| {
        format!("第 {chapter_number} 章的章节分析无效，请重新运行章节分析后再生成：{error}")
    })
}

#[cfg(test)]
mod prior_chapter_analysis_tests {
    use serde_json::{Value, json};

    use super::validate_prior_chapter_context;

    fn valid_analysis() -> Value {
        json!({
            "metadata": {
                "characters": [],
                "locations": [],
                "items": [],
                "tags": [],
                "tone": null,
                "timeline_marker": null
            },
            "summaries": {
                "compressed": "前文摘要",
                "one_line": "前文摘要",
                "keywords": []
            },
            "character_states": {},
            "foreshadowing": {
                "planted": [],
                "resolved": [],
                "tensions": []
            },
            "key_events": []
        })
    }

    #[test]
    fn prior_chapter_with_content_requires_saved_analysis() {
        let error = validate_prior_chapter_context(4, false, "已有正文", None)
            .expect_err("analysis must be required for written prior chapters");

        assert!(error.contains("第 4 章"));
        assert!(error.contains("缺少有效章节分析"));
    }

    #[test]
    fn prior_chapter_analysis_must_match_current_schema() {
        let error = validate_prior_chapter_context(2, false, "已有正文", Some(&json!({})))
            .expect_err("invalid analysis must be rejected");

        assert!(error.contains("第 2 章"));
        assert!(error.contains("章节分析无效"));
    }

    #[test]
    fn valid_prior_chapter_analysis_allows_generation() {
        validate_prior_chapter_context(1, false, "已有正文", Some(&valid_analysis()))
            .expect("current normalized analysis must pass");
    }

    #[test]
    fn empty_prior_chapter_does_not_require_analysis() {
        validate_prior_chapter_context(1, false, "  ", None)
            .expect("empty chapter has no content to analyze");
    }

    #[test]
    fn generating_prior_chapter_blocks_following_generation() {
        let error = validate_prior_chapter_context(3, true, "旧正文", Some(&valid_analysis()))
            .expect_err("a generating prior chapter must not be used as context");

        assert!(error.contains("第 3 章"));
        assert!(error.contains("仍在生成中"));
    }
}
