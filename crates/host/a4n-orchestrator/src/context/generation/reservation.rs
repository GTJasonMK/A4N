//! Generation task reservation and startup.

use super::*;

use super::fingerprints::{
    blueprint_generation_input_fingerprint, blueprint_refinement_input_fingerprint,
    chapter_generation_input_fingerprint, image_generation_input_fingerprint,
    image_generation_operation_kind, image_generation_target_id,
    inspiration_generation_input_fingerprint, optimize_analysis_generation_input_fingerprint,
    optimize_chat_generation_input_fingerprint, optimize_patch_generation_input_fingerprint,
    outline_generation_input_fingerprint, outline_generation_operation_kind,
    outline_generation_target_id, portrait_generation_input_fingerprint,
    portrait_generation_operation_kind, portrait_generation_target_id,
    video_generation_business_key, video_generation_input_fingerprint, video_generation_target_id,
};
use super::read_subscribe::generation_task_handle;

pub(super) struct RootGenerationTaskReservationInput<'a> {
    pub(super) requested_by_user_id: i64,
    pub(super) meta: &'a GenerationStartRequestMeta,
    pub(super) business_key_json: Value,
    pub(super) input_fingerprint_json: Value,
    pub(super) task_kind: TaskKind,
    pub(super) priority: TaskPriority,
    pub(super) delivery_mode: GenerationDeliveryMode,
    pub(super) batch_policy: Option<BatchCommitPolicy>,
    pub(super) project_id: i64,
    pub(super) task_scope: &'static str,
    pub(super) target: GenerationTargetRef,
}

impl GenerationApplication {
    pub(crate) fn new(database: DbHandle, task_queue: Arc<PersistentTaskQueue>) -> Self {
        let registry = Arc::new(
            GenerationSessionRegistry::new(GenerationSessionConfig::default())
                .expect("default generation session config must be valid"),
        );
        let terminal = GenerationTerminalCoordinator::new(database.clone(), registry);
        Self {
            database,
            task_queue,
            terminal,
        }
    }

    pub(super) fn connection(&self) -> &a4n_db::DatabaseConnection {
        self.database.connection()
    }

    async fn initialize_reserved_task(
        &self,
        reservation: a4n_db::generation::GenerationTaskReservation,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let a4n_db::generation::GenerationTaskReservation {
            created,
            task,
            queue_task,
        } = reservation;
        let snapshot = required_public_snapshot(task.clone())
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        let handle = generation_task_handle(&snapshot);
        let queue_pending = matches!(queue_task.status, TaskStatus::Pending);

        if queue_pending && let Err(error) = self.task_queue.register_persisted(queue_task) {
            self.terminate_start_failure(
                &task.checkpoint.task_id,
                task.checkpoint.attempt,
                false,
                error.to_string(),
            )
            .await;
            return Err(GenerationTaskAccessError::Queue(error));
        }

        if snapshot.status.is_terminal()
            || self.session(&snapshot.task_id, snapshot.attempt)?.is_some()
        {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        // A duplicate request can resume only a still-reserved attempt. Running
        // and cancelling attempts belong to an existing executor/recovery path.
        if !created && snapshot.status != GenerationTaskStatus::Reserved {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        let admission = match self.terminal.registry().try_admit() {
            Ok(admission) => admission,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        let started = a4n_db::generation::started_event_from_record(&task.checkpoint)?;
        let started_event = GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: snapshot.task_id.clone(),
            attempt: snapshot.attempt,
            sequence: snapshot.covers_through_sequence,
            emitted_at_ms: snapshot.updated_at_ms,
            event: GenerationEvent::Started(started),
        };
        let reserved_through_sequence =
            match u64::try_from(task.checkpoint.reserved_through_sequence) {
                Ok(value) => value,
                Err(_) => {
                    let error = GenerationSessionError::InvalidSequenceLease {
                        current: snapshot.covers_through_sequence,
                        next: u64::MAX,
                    };
                    self.terminate_start_failure(
                        &task.checkpoint.task_id,
                        task.checkpoint.attempt,
                        queue_pending,
                        error.to_string(),
                    )
                    .await;
                    return Err(GenerationTaskAccessError::Session(error));
                }
            };
        let session = match GenerationTaskSession::from_snapshot(
            snapshot.clone(),
            vec![started_event],
            task.checkpoint.checkpoint_revision,
            reserved_through_sequence,
            snapshot.updated_at_ms,
            admission,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        let registered = match self.terminal.registry().insert_or_get(Arc::clone(&session)) {
            Ok(registered) => registered,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        if !Arc::ptr_eq(&registered, &session) {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        Ok(GenerationTaskStart {
            handle,
            should_run: true,
        })
    }

    async fn terminate_start_failure(
        &self,
        task_id: &str,
        attempt: i32,
        queue_registered: bool,
        summary: String,
    ) {
        if let Ok(attempt) = u32::try_from(attempt) {
            let _ = self
                .terminal
                .interrupt_persisted_task(
                    task_id,
                    attempt,
                    a4n_service_protocol::InterruptionReason::SessionOwnerLost,
                    Some(summary),
                    None,
                )
                .await;
        }
        if queue_registered {
            let _ = self.task_queue.project_generation_terminal(
                task_id,
                TaskStatus::Interrupted {
                    reason: Some("generation start failed".to_string()),
                },
            );
        }
    }

    pub(super) async fn reserve_root_generation_task(
        &self,
        input: RootGenerationTaskReservationInput<'_>,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let RootGenerationTaskReservationInput {
            requested_by_user_id,
            meta,
            business_key_json,
            input_fingerprint_json,
            task_kind,
            priority,
            delivery_mode,
            batch_policy,
            project_id,
            task_scope,
            target,
        } = input;
        let reservation = a4n_db::generation::reserve_generation_task(
            self.connection(),
            ReserveGenerationTaskInput {
                requested_by_user_id,
                start_request_id: meta.start_request_id.clone(),
                business_key_json,
                input_fingerprint_json,
                task_kind,
                priority,
                delivery_mode,
                batch_policy,
                project_id: Some(project_id),
                root_project_id: None,
                source_snapshot_id: None,
                task_scope: Some(task_scope.to_string()),
                workflow_run_id: None,
                generation_run_id: None,
                target: Some(target),
                root_task_id: None,
                parent_task_id: None,
                item_key: None,
                retry_of_task_id: meta.retry_of_task_id.clone(),
            },
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        self.initialize_reserved_task(reservation).await
    }

    pub(crate) async fn start_chapter_generation_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = chapter_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "chapter_generation",
                "project_id": input.project_id,
                "chapter_number": input.chapter_number,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "chapter_generation",
            target: GenerationTargetRef {
                kind: "chapter".to_string(),
                id: format!("{}:{}", input.project_id, input.chapter_number),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_inspiration_generation_task(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = inspiration_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "inspiration_generation",
                "project_id": input.project_id,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "inspiration_generation",
            target: GenerationTargetRef {
                kind: "inspiration_conversation".to_string(),
                id: input.project_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_visual_plan_prompt_generation_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({"version":1,"task_kind":"director_segment_prompt","project_id":input.project_id,"record_id":input.record_id,"segment_id":input.segment_id}),
            input_fingerprint_json: serde_json::json!({"project_id":input.project_id,"source_chapter_id":input.source_chapter_id,"record_id":input.record_id,"expected_revision":input.expected_revision,"segment_id":input.segment_id,"user_description":input.user_description}),
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "director_segment_prompt",
            target: GenerationTargetRef {
                kind: "visual_plan_segment".into(),
                // Keep the source chapter in the durable target identity so a
                // chapter-bound Agent can verify a later wait without replaying
                // the original start arguments.
                id: format!(
                    "chapter:{}:{}:{}",
                    input.source_chapter_id, input.record_id, input.segment_id
                ),
                revision: Some(input.expected_revision.to_string()),
            },
        }).await
    }

    pub(crate) async fn start_shooting_script_generation_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
        fingerprint: Value,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1, "task_kind": "chapter_shooting_script",
                "project_id": input.project_id, "source_chapter_id": input.source_chapter_id,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "chapter_shooting_script",
            target: GenerationTargetRef {
                kind: "chapter_shooting_script".into(),
                id: format!("chapter:{}", input.source_chapter_id),
                revision: Some(input.visual_plan_id.to_string()),
            },
        })
        .await
    }

    pub(crate) async fn start_outline_generation_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = outline_generation_operation_kind(&input.operation);
        let target_id = outline_generation_target_id(input.project_id, &input.operation);
        let fingerprint = outline_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "outline_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "outline_generation",
            target: GenerationTargetRef {
                kind: format!("outline_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_portrait_generation_task(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = portrait_generation_operation_kind(&input.operation);
        let fingerprint = portrait_generation_input_fingerprint(input);
        let batch_policy = matches!(
            &input.operation,
            PortraitGenerationOperation::GenerateMissing { .. }
        )
        .then_some(BatchCommitPolicy::BestEffort);
        let target_id = portrait_generation_target_id(input.project_id, &input.operation);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "portrait_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::ImageGeneration,
            priority: TaskPriority::Medium,
            delivery_mode: GenerationDeliveryMode::StageOnly,
            batch_policy,
            project_id: input.project_id,
            task_scope: "portrait_generation",
            target: GenerationTargetRef {
                kind: format!("portrait_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_image_generation_task(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = image_generation_operation_kind(&input.operation);
        let fingerprint = image_generation_input_fingerprint(input);
        let batch_policy = matches!(
            &input.operation,
            ImageGenerationOperation::Batch { .. }
                | ImageGenerationOperation::VisualPlanPoint { .. }
        )
        .then_some(BatchCommitPolicy::BestEffort);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "image_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::ImageGeneration,
            priority: TaskPriority::Medium,
            delivery_mode: GenerationDeliveryMode::StageOnly,
            batch_policy,
            project_id: input.project_id,
            task_scope: "image_generation",
            target: GenerationTargetRef {
                kind: format!("image_{operation_kind}"),
                id: image_generation_target_id(input.project_id, &input.operation),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_video_generation_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = input.operation.kind();
        let target_id = video_generation_target_id(input.project_id, &input.operation)
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        let fingerprint = video_generation_input_fingerprint(input);
        let prompt_operation = input.operation.is_prompt_generation();
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: video_generation_business_key(
                input.project_id,
                operation_kind,
                &target_id,
            ),
            input_fingerprint_json: fingerprint,
            task_kind: if prompt_operation {
                TaskKind::LlmCall
            } else {
                TaskKind::VideoGeneration
            },
            priority: if prompt_operation {
                TaskPriority::High
            } else {
                TaskPriority::Medium
            },
            delivery_mode: if prompt_operation {
                GenerationDeliveryMode::Stream
            } else {
                GenerationDeliveryMode::StageOnly
            },
            batch_policy: match &input.operation {
                VideoGenerationOperation::ChapterBatch { .. }
                | VideoGenerationOperation::PromptFrames { .. }
                | VideoGenerationOperation::PromptVideos { .. } => {
                    Some(BatchCommitPolicy::BestEffort)
                }
                _ => None,
            },
            project_id: input.project_id,
            task_scope: if prompt_operation {
                "video_prompt_generation"
            } else {
                "video_generation"
            },
            target: GenerationTargetRef {
                kind: format!("video_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_blueprint_generation_task(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = blueprint_generation_input_fingerprint(input);
        self.start_blueprint_mutation_task(
            user_id,
            &input.meta,
            input.project_id,
            fingerprint,
            BLUEPRINT_GENERATION_KIND,
        )
        .await
    }

    pub(crate) async fn start_blueprint_refinement_task(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = blueprint_refinement_input_fingerprint(input);
        self.start_blueprint_mutation_task(
            user_id,
            &input.meta,
            input.project_id,
            fingerprint,
            BLUEPRINT_REFINEMENT_TASK_KIND,
        )
        .await
    }

    async fn start_blueprint_mutation_task(
        &self,
        user_id: i64,
        meta: &GenerationStartRequestMeta,
        project_id: i64,
        fingerprint: Value,
        task_scope: &'static str,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "blueprint_mutation",
                "project_id": project_id,
                "operation_mode": "blueprint",
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope,
            target: GenerationTargetRef {
                kind: task_scope.to_string(),
                id: project_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_chat_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = optimize_chat_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_chat",
                "session_id": input.request.session_id,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_chat",
            target: GenerationTargetRef {
                kind: "optimization_chat".to_string(),
                id: input.request.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let input_fingerprint_json = optimize_analysis_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_analysis",
                "session_id": input.session_id,
            }),
            input_fingerprint_json,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_analysis",
            target: GenerationTargetRef {
                kind: "optimization_analysis".to_string(),
                id: input.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_patch_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let input_fingerprint_json = optimize_patch_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_patch",
                "session_id": input.request.session_id,
            }),
            input_fingerprint_json,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_patch",
            target: GenerationTargetRef {
                kind: "optimization_patch".to_string(),
                id: input.request.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_manga_pipeline_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::manga::MangaPipelineStartInput,
        chapter_id: i64,
        fingerprint: Value,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: input.meta.as_ref().ok_or_else(|| {
                GenerationTaskAccessError::InvalidArgument(
                    "漫画流水线缺少 generation start meta".to_string(),
                )
            })?,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "manga_pipeline",
                "project_id": input.project_id,
                "chapter_number": input.chapter_number,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::MangaGeneration,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "manga_pipeline",
            target: GenerationTargetRef {
                kind: "manga_pipeline".into(),
                id: format!("chapter:{chapter_id}"),
                revision: None,
            },
        })
        .await
    }
}

pub(super) fn required_public_snapshot(
    task: a4n_db::generation::PersistedGenerationTask,
) -> Result<GenerationTaskSnapshot, AppError> {
    task.public_snapshot.ok_or_else(|| AppError::InvalidState {
        current: format!(
            "generation task {} 缺少 public snapshot",
            task.checkpoint.task_id
        ),
        operation: "初始化 generation session".to_string(),
    })
}
