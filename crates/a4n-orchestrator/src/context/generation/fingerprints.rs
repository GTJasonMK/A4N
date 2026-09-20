//! Generation input fingerprints and video-start validation.

use super::*;

impl OrchestratorContext {
    pub(super) async fn validate_video_generation_start(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<(), AppError> {
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        match &input.operation {
            VideoGenerationOperation::Chapter {
                chapter_id,
                prompt_id,
                config_id,
            } => {
                if *chapter_id <= 0 || *prompt_id <= 0 || config_id.is_some_and(|id| id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "章节视频的 chapter_id、prompt_id、config_id 必须为正数".to_string(),
                    ));
                }
            }
            VideoGenerationOperation::ChapterBatch {
                chapter_id,
                prompt_ids,
                config_id,
            } => {
                if *chapter_id <= 0 || prompt_ids.is_empty() || config_id.is_some_and(|id| id <= 0)
                {
                    return Err(AppError::InvalidArgument(
                        "批量视频必须包含 chapter_id 和 prompt_ids".to_string(),
                    ));
                }
                if prompt_ids.iter().any(|id| *id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "批量视频 prompt_id 必须为正数".to_string(),
                    ));
                }
                let mut unique = prompt_ids.clone();
                unique.sort_unstable();
                unique.dedup();
                if unique.len() != prompt_ids.len() {
                    return Err(AppError::InvalidArgument(
                        "批量视频 prompt_ids 不能重复".to_string(),
                    ));
                }
            }
            VideoGenerationOperation::ReferenceMedia { chapter_id, input } => {
                if *chapter_id <= 0 {
                    return Err(AppError::InvalidArgument(
                        "参考媒体视频必须包含 chapter_id".to_string(),
                    ));
                }
                if input.config_id.is_some_and(|id| id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "参考媒体视频的 config_id 无效".to_string(),
                    ));
                }
                validate_reference_media_start_input(input).map_err(AppError::InvalidArgument)?;
            }
            VideoGenerationOperation::PromptSegmentPlan {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_segment_plan_generation_options,
            )?,
            VideoGenerationOperation::PromptInsertedSegment { options, .. } => {
                crate::video::validate_video_inserted_segment_generation_options(options)?
            }
            VideoGenerationOperation::PromptFrames {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_frame_prompt_generation_options,
            )?,
            VideoGenerationOperation::PromptVideos {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_final_prompt_generation_options,
            )?,
        }
        Ok(())
    }
}

pub(super) fn chapter_generation_input_fingerprint(input: &StartChapterGenerationInput) -> Value {
    serde_json::json!({
        "version": 2,
        "task_kind": "chapter_generation",
        "project_id": input.project_id,
        "chapter_number": input.chapter_number,
        "writing_notes": input.writing_notes,
        "min_word_count": input.min_word_count,
        "max_word_count": input.max_word_count,
        "client_rag_embedding_identity": input.client_rag_embedding_identity,
        "client_rag_query_fingerprint": input.client_rag_query_fingerprint,
    })
}

pub(super) fn inspiration_generation_input_fingerprint(
    input: &StartInspirationGenerationInput,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "inspiration_generation",
        "project_id": input.project_id,
        "message": input.message,
        "variables": null,
    })
}

pub(super) fn outline_generation_input_fingerprint(input: &StartOutlineGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "outline_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

pub(super) fn outline_generation_operation_kind(
    operation: &OutlineGenerationOperation,
) -> &'static str {
    match operation {
        OutlineGenerationOperation::GenerateParts { .. } => "generate_parts",
        OutlineGenerationOperation::RegenerateParts { .. } => "regenerate_parts",
        OutlineGenerationOperation::RegenerateLastPart { .. } => "regenerate_last_part",
        OutlineGenerationOperation::ContinuePartOutlines { .. } => "continue_part_outlines",
        OutlineGenerationOperation::RegeneratePartOutline { .. } => "regenerate_part_outline",
        OutlineGenerationOperation::GeneratePartChapterOutlines { .. } => {
            "generate_part_chapter_outlines"
        }
        OutlineGenerationOperation::BatchGeneratePartOutlines { .. } => {
            "batch_generate_part_outlines"
        }
        OutlineGenerationOperation::GenerateChapterOutlinesByCount { .. } => {
            "generate_chapter_outlines_by_count"
        }
        OutlineGenerationOperation::RegenerateChapterOutline { .. } => "regenerate_chapter_outline",
        OutlineGenerationOperation::ApplyBeatMapping { .. } => "apply_beat_mapping",
    }
}

pub(super) fn outline_generation_target_id(
    project_id: i64,
    operation: &OutlineGenerationOperation,
) -> String {
    match operation {
        OutlineGenerationOperation::RegeneratePartOutline { part_id, .. }
        | OutlineGenerationOperation::GeneratePartChapterOutlines { part_id }
        | OutlineGenerationOperation::ApplyBeatMapping { part_id, .. } => {
            format!("{project_id}:part:{part_id}")
        }
        OutlineGenerationOperation::RegenerateChapterOutline { outline_id, .. } => {
            format!("{project_id}:outline:{outline_id}")
        }
        _ => project_id.to_string(),
    }
}

pub(super) fn portrait_generation_input_fingerprint(input: &StartPortraitGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "portrait_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

pub(super) fn portrait_generation_operation_kind(
    operation: &PortraitGenerationOperation,
) -> &'static str {
    match operation {
        PortraitGenerationOperation::Generate { .. } => "generate",
        PortraitGenerationOperation::Regenerate { .. } => "regenerate",
        PortraitGenerationOperation::GenerateAgeVariant { .. } => "age_variant",
        PortraitGenerationOperation::GenerateMissing { .. } => "generate_missing",
    }
}

pub(super) fn portrait_generation_target_id(
    project_id: i64,
    operation: &PortraitGenerationOperation,
) -> String {
    match operation {
        PortraitGenerationOperation::Regenerate { portrait_id, .. }
        | PortraitGenerationOperation::GenerateAgeVariant { portrait_id, .. } => {
            format!("{project_id}:portrait:{portrait_id}")
        }
        _ => project_id.to_string(),
    }
}

pub(super) fn image_generation_input_fingerprint(input: &StartImageGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "image_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

pub(super) fn image_generation_operation_kind(
    operation: &ImageGenerationOperation,
) -> &'static str {
    match operation {
        ImageGenerationOperation::VisualPlanPoint { .. } => "visual_plan_point",
        ImageGenerationOperation::Scene { .. } => "scene",
        ImageGenerationOperation::Page { .. } => "page",
        ImageGenerationOperation::Batch { .. } => "batch",
        ImageGenerationOperation::ObjectProfile { .. } => "object_profile",
        ImageGenerationOperation::SceneProfile { .. } => "scene_profile",
    }
}

pub(super) fn image_generation_target_id(
    project_id: i64,
    operation: &ImageGenerationOperation,
) -> String {
    match operation {
        ImageGenerationOperation::VisualPlanPoint { options } => format!(
            "{project_id}:visual_plan:chapter:{}:{}:{}",
            options.source_chapter_id, options.record_id, options.point_id
        ),
        ImageGenerationOperation::ObjectProfile {
            object_profile_id,
            chapter_id: Some(chapter_id),
            ..
        } => format!("{project_id}:object_profile:chapter:{chapter_id}:{object_profile_id}"),
        ImageGenerationOperation::ObjectProfile {
            object_profile_id, ..
        } => format!("{project_id}:object_profile:{object_profile_id}"),
        ImageGenerationOperation::SceneProfile {
            scene_profile_id,
            chapter_id: Some(chapter_id),
            ..
        } => format!("{project_id}:scene_profile:chapter:{chapter_id}:{scene_profile_id}"),
        ImageGenerationOperation::SceneProfile {
            scene_profile_id, ..
        } => format!("{project_id}:scene_profile:{scene_profile_id}"),
        ImageGenerationOperation::Scene {
            chapter_id: Some(chapter_id),
            ..
        } => format!("{project_id}:scene:chapter:{chapter_id}"),
        _ => project_id.to_string(),
    }
}

pub(super) fn video_generation_input_fingerprint(input: &StartVideoGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "video_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

pub(super) fn video_generation_business_key(
    project_id: i64,
    operation_kind: &str,
    target_id: &str,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "video_generation",
        "project_id": project_id,
        "operation_kind": operation_kind,
        "target_id": target_id,
    })
}

pub(super) fn video_generation_target_id(
    project_id: i64,
    operation: &VideoGenerationOperation,
) -> Result<String, TimelineError> {
    Ok(match operation {
        VideoGenerationOperation::Chapter {
            chapter_id,
            prompt_id,
            ..
        } => format!("{project_id}:chapter:{chapter_id}:prompt:{prompt_id}"),
        VideoGenerationOperation::ChapterBatch { chapter_id, .. } => {
            format!("{project_id}:chapter:{chapter_id}")
        }
        VideoGenerationOperation::ReferenceMedia { chapter_id, input } => {
            format!(
                "{project_id}:chapter:{chapter_id}:reference_media:plan:{}:shot:{}:index:{}",
                input.segment_plan_prompt_id,
                input.target_segment.source_shot_id,
                input.target_segment.segment_index_in_shot,
            )
        }
        VideoGenerationOperation::PromptSegmentPlan { chapter_id, .. } => {
            format!("{project_id}:chapter:{chapter_id}:prompt_segment_plan:shared")
        }
        VideoGenerationOperation::PromptInsertedSegment {
            chapter_id,
            options,
        } => {
            let gap_key = VideoTimelineGap {
                previous_item_id: options.previous_item_id,
                next_item_id: options.next_item_id,
            }
            .canonical_key()?;
            format!(
                "{project_id}:chapter:{chapter_id}:prompt_inserted_segment:timeline:{}:gap:{gap_key}",
                options.timeline_id,
            )
        }
        VideoGenerationOperation::PromptFrames {
            chapter_id,
            options,
        }
        | VideoGenerationOperation::PromptVideos {
            chapter_id,
            options,
        } => video_prompt_unit_target_id(project_id, *chapter_id, operation.kind(), options),
    })
}

fn video_prompt_unit_target_id(
    project_id: i64,
    chapter_id: i64,
    operation_kind: &str,
    options: &VideoPromptGenerationOptions,
) -> String {
    let segment_plan_prompt_id = options
        .segment_plan_prompt_id
        .expect("video prompt start validation requires segment plan");
    let target_segment = options
        .target_segment
        .as_ref()
        .expect("video prompt start validation requires target segment");
    format!(
        "{project_id}:chapter:{chapter_id}:{operation_kind}:plan:{segment_plan_prompt_id}:shot:{}:index:{}",
        target_segment.source_shot_id, target_segment.segment_index_in_shot,
    )
}

fn validate_video_prompt_generation_start(
    chapter_id: i64,
    options: &VideoPromptGenerationOptions,
    validate_options: fn(&VideoPromptGenerationOptions) -> Result<(), AppError>,
) -> Result<(), AppError> {
    if chapter_id <= 0 {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 chapter_id 必须为正数".to_string(),
        ));
    }
    validate_options(options)
}

pub(super) fn blueprint_generation_input_fingerprint(
    input: &StartBlueprintGenerationInput,
) -> Value {
    let variables = input
        .variables
        .as_ref()
        .map(|value| serde_json::to_value(value).expect("BTreeMap values serialize"))
        .unwrap_or_else(|| serde_json::json!({}));
    canonical_blueprint_generation_input_fingerprint(
        input.project_id,
        &input.title,
        &input.genre,
        input.target_length,
        &variables,
    )
}

pub(super) fn blueprint_refinement_input_fingerprint(
    input: &StartBlueprintRefinementInput,
) -> Value {
    let variables = input
        .variables
        .as_ref()
        .map(|value| serde_json::to_value(value).expect("BTreeMap values serialize"))
        .unwrap_or_else(|| serde_json::json!({}));
    canonical_blueprint_refinement_input_fingerprint(
        input.project_id,
        input.focus.as_deref(),
        &variables,
    )
}

pub(super) fn optimize_chat_generation_input_fingerprint(
    input: &StartOptimizeChatGenerationInput,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_chat",
        "session_id": input.request.session_id,
        "message": input.request.message,
        "permissions": input.request.permissions,
    })
}

pub(super) fn optimize_analysis_generation_input_fingerprint(
    input: &StartOptimizeAnalysisGenerationInput,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_analysis",
        "session_id": input.session_id,
    })
}

pub(super) fn optimize_patch_generation_input_fingerprint(
    input: &StartOptimizePatchGenerationInput,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_patch",
        "session_id": input.request.session_id,
        "issue_ids": input.request.issue_ids,
        "agent_instruction": input.request.agent_instruction,
    })
}

pub(super) fn map_optimize_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

pub(super) fn map_outline_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

pub(super) fn map_portrait_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

pub(super) fn map_image_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

pub(super) fn map_video_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

#[cfg(test)]
mod video_prompt_generation_tests {
    use super::super::reservation::RootGenerationTaskReservationInput;
    use super::*;
    use a4n_api_contract::{
        generation_tasks::GenerationStartRequestMeta,
        video::{ReferenceMediaVideoInput, VideoPromptGenerationOptions, VideoSegmentTarget},
    };

    fn options() -> VideoPromptGenerationOptions {
        VideoPromptGenerationOptions {
            workflow_mode: Some("storyboard".to_string()),
            segment_plan_prompt_id: Some(11),
            style: "电影感".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            prompt_batch_size: 2,
            target_segment: Some(VideoSegmentTarget {
                source_shot_id: "shot-4".to_string(),
                segment_index_in_shot: 1,
            }),
            target_narrative_unit_index: None,
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: vec![1],
            video_library_image_ids: vec![2],
            regenerate_existing: false,
        }
    }

    fn input(options: VideoPromptGenerationOptions) -> StartVideoGenerationInput {
        StartVideoGenerationInput {
            meta: GenerationStartRequestMeta {
                start_request_id: "request-1".to_string(),
                retry_of_task_id: None,
            },
            project_id: 19,
            operation: VideoGenerationOperation::PromptFrames {
                chapter_id: 3,
                options,
            },
        }
    }

    fn validate_start(options: &VideoPromptGenerationOptions) -> Result<(), AppError> {
        validate_video_prompt_generation_start(
            3,
            options,
            crate::video::validate_video_frame_prompt_generation_options,
        )
    }

    #[test]
    fn prompt_target_contains_chapter_operation_and_workflow() {
        let operation = input(options()).operation;
        assert_eq!(
            video_generation_target_id(19, &operation).unwrap(),
            "19:chapter:3:prompt_frames:plan:11:shot:shot-4:index:1"
        );
    }

    #[test]
    fn prompt_business_key_uses_canonical_target() {
        let input = input(options());
        let target_id = video_generation_target_id(input.project_id, &input.operation).unwrap();
        assert_eq!(
            video_generation_business_key(input.project_id, input.operation.kind(), &target_id,),
            serde_json::json!({
                "version": 1,
                "task_kind": "video_generation",
                "project_id": 19,
                "operation_kind": "prompt_frames",
                "target_id": "19:chapter:3:prompt_frames:plan:11:shot:shot-4:index:1",
            })
        );
    }

    #[test]
    fn reference_media_target_identifies_plan_and_segment() {
        let operation = VideoGenerationOperation::ReferenceMedia {
            chapter_id: 3,
            input: ReferenceMediaVideoInput {
                segment_plan_prompt_id: 11,
                target_segment: VideoSegmentTarget {
                    source_shot_id: "shot-4".to_string(),
                    segment_index_in_shot: 1,
                },
                image_uploads: vec![],
                video_uploads: vec![],
                audio_uploads: vec![],
                config_id: None,
            },
        };
        assert_eq!(
            video_generation_target_id(19, &operation).unwrap(),
            "19:chapter:3:reference_media:plan:11:shot:shot-4:index:1"
        );
    }

    #[test]
    fn prompt_fingerprint_changes_with_options() {
        let first = input(options());
        let mut changed_options = options();
        changed_options.style = "水彩".to_string();
        let second = input(changed_options);
        assert_ne!(
            video_generation_input_fingerprint(&first),
            video_generation_input_fingerprint(&second)
        );
    }

    #[test]
    fn prompt_start_validation_rejects_missing_contract_fields() {
        let mut invalid = options();
        invalid.workflow_mode = Some(" ".to_string());
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.style.clear();
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.prompt_batch_size = 0;
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.duration_secs = 0;
        assert!(validate_start(&invalid).is_err());
    }

    #[tokio::test]
    async fn workflow_binding_refreshes_live_session_before_cancel() {
        let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
        a4n_db::package::ensure_host_package_states(
            database.as_ref(),
            &["ai-workflow".into(), "video".into()],
        )
        .await
        .unwrap();
        let context = crate::test_support::TestOrchestratorBuilder::new(Arc::clone(&database))
            .build()
            .await;
        let user_id = crate::test_support::create_test_user(database.as_ref()).await;
        let project = a4n_db::project::create_project_record(
            database.as_ref(),
            a4n_db::project::CreateProjectRecordInput {
                user_id,
                title: "workflow binding cancellation".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let meta = GenerationStartRequestMeta {
            start_request_id: uuid::Uuid::new_v4().to_string(),
            retry_of_task_id: None,
        };
        let start = context
            .generation
            .reserve_root_generation_task(RootGenerationTaskReservationInput {
                requested_by_user_id: user_id,
                meta: &meta,
                business_key_json: serde_json::json!({"version": 1, "test": "workflow-binding"}),
                input_fingerprint_json: serde_json::json!({"test": "workflow-binding"}),
                task_kind: TaskKind::LlmCall,
                priority: TaskPriority::High,
                delivery_mode: GenerationDeliveryMode::Stream,
                batch_policy: None,
                project_id: project.id,
                task_scope: "video_prompt_generation",
                target: GenerationTargetRef {
                    kind: "video_prompt_segment_plan".to_string(),
                    id: "workflow-binding-test".to_string(),
                    revision: None,
                },
            })
            .await
            .unwrap();
        assert!(start.should_run);
        let task_id = start.handle.task_id.clone();
        let workflow = context
            .claim_exclusive_workflow_run(crate::workflow::ClaimExclusiveWorkflowRunInput {
                run: crate::workflow::UpsertWorkflowRunInput {
                    business_key: "workflow-binding-cancellation".to_string(),
                    workflow_kind: "video_prompts".to_string(),
                    project_id: Some(project.id),
                    root_project_id: Some(project.id),
                    source_snapshot_id: None,
                    scope_table: Some("chapter".to_string()),
                    scope_id: Some(1),
                    chapter_id: Some(1),
                    mode: Some("storyboard".to_string()),
                    input_fingerprint_json: Some(serde_json::json!({"version": 1})),
                    status: crate::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
                    current_step_key: Some("video_prompts".to_string()),
                    active_task_id: Some(task_id.clone()),
                    can_resume: false,
                    resume_block_reason: None,
                    error_message: None,
                },
                resume_from_task_id: None,
            })
            .await
            .unwrap();
        let session = context
            .generation
            .session(&task_id, start.handle.attempt)
            .unwrap()
            .unwrap();
        assert_eq!(
            session.snapshot().started.workflow_run_id,
            Some(workflow.id)
        );
        assert_eq!(session.checkpoint_candidate().checkpoint_revision, 2);

        let execution = context
            .generation
            .acquire_task_execution(&start.handle)
            .await
            .unwrap()
            .unwrap();
        let cancelled = context
            .request_generation_cancel(user_id, &task_id, start.handle.attempt)
            .await
            .unwrap();
        assert_eq!(cancelled.status, GenerationTaskStatus::Cancelling);
        drop(execution);
    }
}
