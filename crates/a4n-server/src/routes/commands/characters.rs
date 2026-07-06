use super::*;

macro_rules! character_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state, user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

character_web_input_command!(
    create_profile,
    character_records_actions::CreateProfileInput,
    character_records_actions::create_profile,
);

character_web_input_command!(
    list_profiles,
    character_records_actions::ListProfilesInput,
    character_records_actions::list_profiles,
);

pub(super) async fn delete_profile(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let profile_id = i64_arg(args, "profileId")?;
    let result = character_records_actions::delete_profile(state, user_id, profile_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

character_web_input_command!(
    add_profile_attribute,
    character_records_actions::AddProfileAttributeInput,
    character_records_actions::add_profile_attribute,
);

character_web_input_command!(
    modify_profile_attribute,
    character_records_actions::ModifyProfileAttributeInput,
    character_records_actions::modify_profile_attribute,
);

character_web_input_command!(
    attribute_history,
    character_records_actions::AttributeHistoryInput,
    character_records_actions::attribute_history,
);

pub(super) async fn get_character_arc_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let value =
        character_records_actions::get_character_arc_status(state, user_id, project_id, name)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(value)
}

pub(super) async fn get_character_voice_fingerprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let value = character_records_actions::get_character_voice_fingerprint(
        state, user_id, project_id, name,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(value)
}

macro_rules! character_ai_web_task_command {
    (
        $name:ident,
        |$state:ident, $user_id:ident, $args:ident, $project_id:ident, $progress:ident|,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        ($($arg:ident : $arg_ty:ty = $parse:expr),* $(,)?),
        $run:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let $project_id = i64_arg($args, "projectId")?;
            $(let $arg: $arg_ty = $parse;)*
            let scope = $scope($state, $user_id, $project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter($state, task, task_id.clone());
            let runtime_progress = character_ai_actions::CharacterAiProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
            );
            let $progress = &runtime_progress;
            let result = $run.await;
            let value = finish_web_task(
                $state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(value)
        }
    };
}

macro_rules! character_ai_web_preview_command {
    (
        $name:ident,
        |$state:ident, $user_id:ident, $args:ident, $project_id:ident|,
        ($($arg:ident : $arg_ty:ty = $parse:expr),* $(,)?),
        $preview:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let $project_id = i64_arg($args, "projectId")?;
            $(let $arg: $arg_ty = $parse;)*
            to_value($preview.await.map_err(ApiError::bad_request)?)
        }
    };
}

character_ai_web_task_command!(
    discover_character_profiles,
    |state, user_id, args, project_id, progress|,
    character_ai_actions::character_discovery_task_scope,
    "正在准备角色发现",
    "角色发现完成",
    (chapter_number: Option<i32> = optional_i32_field(args, "chapterNumber")?),
    character_ai_actions::discover_character_profiles_for_chapters(
        state,
        user_id,
        project_id,
        chapter_number,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_discover_character_profiles_request,
    |state, user_id, args, project_id|,
    (chapter_number: Option<i32> = optional_i32_field(args, "chapterNumber")?),
    character_ai_actions::preview_discover_character_profiles_request(
        state,
        user_id,
        project_id,
        chapter_number,
    ),
);

character_ai_web_task_command!(
    sync_all_character_profiles,
    |state, user_id, args, project_id, progress|,
    character_ai_actions::character_profile_sync_all_task_scope,
    "正在准备批量角色同步",
    "批量角色同步完成",
    (chapter_number: i32 =
        required_json_i32(args, "chapterNumber", "sync_all_character_profiles")?),
    character_ai_actions::sync_all_character_profiles_to_chapter(
        state,
        user_id,
        project_id,
        chapter_number,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_sync_all_character_profiles_request,
    |state, user_id, args, project_id|,
    (chapter_number: i32 = required_json_i32(
        args,
        "chapterNumber",
        "preview_sync_all_character_profiles_request",
    )?),
    character_ai_actions::preview_sync_all_character_profiles_request(
        state,
        user_id,
        project_id,
        chapter_number,
    ),
);

character_ai_web_task_command!(
    sync_character_from_chapters,
    |state, user_id, args, project_id, progress|,
    character_ai_actions::character_profile_sync_task_scope,
    "正在准备角色同步",
    "角色同步完成",
    (
        name: String = string_arg(args, "name")?,
        chapter_number: i32 =
            required_json_i32(args, "chapterNumber", "sync_character_from_chapters")?,
    ),
    character_ai_actions::sync_character_profile_from_chapters(
        state,
        user_id,
        project_id,
        &name,
        chapter_number,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_sync_character_from_chapters_request,
    |state, user_id, args, project_id|,
    (
        name: String = string_arg(args, "name")?,
        chapter_number: i32 = required_json_i32(
            args,
            "chapterNumber",
            "preview_sync_character_from_chapters_request",
        )?,
    ),
    character_ai_actions::preview_sync_character_from_chapters_request(
        state,
        user_id,
        project_id,
        &name,
        chapter_number,
    ),
);

pub(super) async fn check_character_conflict(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let start_chapter = optional_i32_field(args, "startChapter")?;
    let end_chapter = optional_i32_field(args, "endChapter")?;
    let scope =
        character_conflict_actions::character_conflict_task_scope(state, user_id, project_id)
            .await
            .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备角色一致性检查",
    )
    .await;
    let progress = web_progress_reporter(state, task, task_id.clone());
    let result = character_conflict_actions::check_character_conflict(
        state,
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
        &progress,
    )
    .await;
    let rows = finish_web_task(state, task_id, result, "一致性检查完成").await?;
    Ok(json!(rows))
}

pub(super) async fn preview_check_character_conflict_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let start_chapter = optional_i32_field(args, "startChapter")?;
    let end_chapter = optional_i32_field(args, "endChapter")?;
    to_value(
        character_conflict_actions::preview_character_conflict_request(
            state,
            user_id,
            project_id,
            &name,
            start_chapter,
            end_chapter,
        )
        .await?,
    )
}

macro_rules! optimize_web_session_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            Ok(json!(
                $action(state, user_id, session_id)
                    .await
                    .map_err(ApiError::bad_request)?
            ))
        }
    };
}

macro_rules! optimize_web_session_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            $action(state, user_id, session_id)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! optimize_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            Ok(json!(
                $action(state, user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?
            ))
        }
    };
}

macro_rules! optimize_web_session_stream_task_command {
    ($name:ident, $scope:path, $start_message:expr, $success_message:expr, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            let scope = $scope(state, user_id, session_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let mut report_stream = web_chunk_reporter(state, task, task_id.clone());
            let result = $action(
                state,
                user_id,
                session_id,
                Some(&mut report_progress),
                Some(&mut report_stream),
            )
            .await;
            let value = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

macro_rules! optimize_web_input_stream_task_command {
    (
        $name:ident,
        $input:ty,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let scope = $scope(state, user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let mut report_stream = web_chunk_reporter(state, task, task_id.clone());
            let result = $action(
                state,
                user_id,
                input,
                Some(&mut report_progress),
                Some(&mut report_stream),
            )
            .await;
            let value = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

macro_rules! optimize_web_input_progress_task_command {
    (
        $name:ident,
        $input:ty,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let scope = $scope(state, user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task, task_id.clone());
            let result = $action(state, user_id, input, Some(&mut report_progress)).await;
            let value = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

macro_rules! optimize_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let preview = $action(state, user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

pub(super) async fn list_optimize_sessions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = nullable_i64_arg(args, "chapterId")?;
    Ok(json!(
        optimize_actions::list_optimize_sessions(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

optimize_web_session_command!(get_optimize_session, optimize_actions::get_optimize_session);
optimize_web_session_command!(list_optimize_turns, optimize_actions::list_optimize_turns);
optimize_web_session_command!(
    list_optimize_patch_sets,
    optimize_actions::list_optimize_patch_sets,
);
optimize_web_session_unit_command!(
    reject_pending_optimize_patch_sets,
    optimize_actions::reject_pending_optimize_patch_sets,
);
optimize_web_session_unit_command!(
    delete_optimize_session,
    optimize_actions::delete_optimize_session,
);

pub(super) async fn record_optimize_agent_event(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let session_id = i64_arg(args, "sessionId")?;
    let content = string_arg(args, "content")?;
    optimize_actions::record_optimize_agent_event(state, user_id, session_id, content)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

optimize_web_input_command!(
    create_optimization_chat_session,
    optimize_actions::CreateOptimizationChatSessionInput,
    optimize_actions::create_optimization_chat_session,
);
optimize_web_input_command!(
    create_optimization_analysis_session,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::create_optimization_analysis_session,
);
optimize_web_input_command!(
    prepare_optimization_analysis_session,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::prepare_optimization_analysis_session,
);
optimize_web_session_stream_task_command!(
    execute_optimization_analysis_session,
    optimize_actions::execute_optimization_analysis_session_task_scope,
    "正在准备执行内容优化分析",
    "优化分析完成",
    optimize_actions::execute_optimization_analysis_session,
);
optimize_web_preview_command!(
    preview_create_optimization_analysis_session_request,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::preview_create_optimization_analysis_session_request,
);
optimize_web_preview_command!(
    preview_prepare_optimization_analysis_session_request,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::preview_prepare_optimization_analysis_session_request,
);
optimize_web_input_stream_task_command!(
    propose_optimization_patches,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::propose_optimization_patches_task_scope,
    "正在准备局部修复补丁",
    "局部补丁生成完成",
    optimize_actions::propose_optimization_patches,
);
optimize_web_preview_command!(
    preview_propose_optimization_patches_request,
    optimize_actions::ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);
optimize_web_input_progress_task_command!(
    apply_optimization_patches,
    optimize_actions::ApplyOptimizationPatchesInput,
    optimize_actions::apply_optimization_patches_task_scope,
    "正在应用局部补丁",
    "局部补丁应用完成",
    optimize_actions::apply_optimization_patches,
);
optimize_web_input_stream_task_command!(
    send_optimization_message,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::send_optimization_message_task_scope,
    "正在准备内容优化 Agent 对话",
    "内容优化 Agent 已回复",
    optimize_actions::send_optimization_message,
);
optimize_web_preview_command!(
    preview_send_optimization_message_request,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);

pub(super) async fn get_inspiration_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let limit = u64_arg(args, "limit")?;
    to_value(
        inspiration_records_actions::get_inspiration_history(state, user_id, project_id, limit)
            .await?,
    )
}

pub(super) async fn converse_inspiration(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::ConverseInput =
        command_input(args, "converse_inspiration")?;
    Ok(json!(
        inspiration_records_actions::converse_inspiration(state, user_id, input).await?
    ))
}

pub(super) async fn converse_inspiration_stream(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::ConverseStreamInput =
        command_input(args, "converse_inspiration_stream")?;
    let task_id = inspiration_records_actions::inspiration_stream_task_id(
        input.project_id,
        input.task_id.clone(),
    )?;
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<(String, u32)>();
    let chunk_task_id = task_id.clone();
    let chunk_state = (*state).clone();
    let chunk_worker = tokio::spawn(async move {
        while let Some((content, index)) = chunk_rx.recv().await {
            emit_workflow_chunk(&chunk_state, &chunk_task_id, &content, index).await;
        }
    });

    let result = inspiration_records_actions::converse_inspiration_stream(
        state,
        user_id,
        input,
        move |delta, index| {
            let _ = chunk_tx.send((delta.to_string(), index));
        },
    )
    .await;
    let _ = chunk_worker.await;

    match result {
        Ok(full) => {
            emit_workflow_complete(state, &task_id, Some(full.clone())).await;
            Ok(json!(full))
        }
        Err(err) => {
            let message = err.to_string();
            emit_workflow_error(state, &task_id, "stream_error", &message, false).await;
            Err(err.into())
        }
    }
}

pub(super) async fn preview_inspiration_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::PreviewInspirationRequestInput =
        command_input(args, "preview_inspiration_request")?;
    to_value(inspiration_records_actions::preview_inspiration_request(state, user_id, input).await?)
}

character_web_input_command!(
    list_snapshots,
    character_records_actions::ListSnapshotsInput,
    character_records_actions::list_snapshots,
);

character_web_input_command!(
    get_profile_state_at_chapter,
    character_records_actions::GetProfileStateAtChapterInput,
    character_records_actions::get_profile_state_at_chapter,
);

character_web_input_command!(
    get_snapshot,
    character_records_actions::GetSnapshotInput,
    character_records_actions::get_snapshot,
);

character_web_input_command!(
    diff_profile_chapters,
    character_records_actions::DiffProfileChaptersInput,
    character_records_actions::diff_profile_chapters,
);

character_web_input_command!(
    check_profile_sync_conflict,
    character_records_actions::CheckProfileSyncConflictInput,
    character_records_actions::check_profile_sync_conflict,
);

character_web_input_command!(
    rollback_to_chapter,
    character_records_actions::RollbackToChapterInput,
    character_records_actions::rollback_to_chapter,
);

character_web_input_command!(
    list_behaviors,
    character_records_actions::ListBehaviorsInput,
    character_records_actions::list_behaviors,
);

character_web_input_command!(
    get_implicit_review,
    character_implicit_review_actions::GetImplicitReviewInput,
    character_implicit_review_actions::get_implicit_review,
);

character_web_input_command!(
    check_implicit_update,
    character_implicit_review_actions::CheckImplicitUpdateInput,
    character_implicit_review_actions::check_implicit_update,
);

pub(super) async fn preview_check_implicit_update_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: character_implicit_review_actions::CheckImplicitUpdateInput =
        command_input(args, "preview_check_implicit_update_request")?;
    let preview = character_implicit_review_actions::preview_check_implicit_update_request(
        state, user_id, &input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

character_web_input_command!(
    apply_implicit_update_decision,
    character_implicit_review_actions::ApplyImplicitUpdateDecisionInput,
    character_implicit_review_actions::apply_implicit_update_decision,
);

character_web_input_command!(
    list_deletion_marks,
    character_records_actions::ListDeletionMarksInput,
    character_records_actions::list_deletion_marks,
);

character_web_input_command!(
    request_attribute_deletion,
    character_records_actions::RequestAttributeDeletionInput,
    character_records_actions::request_attribute_deletion,
);

character_web_input_command!(
    execute_attribute_deletion,
    character_records_actions::ExecuteAttributeDeletionInput,
    character_records_actions::execute_attribute_deletion,
);

character_web_input_command!(
    reset_deletion_marks,
    character_records_actions::ResetDeletionMarksInput,
    character_records_actions::reset_deletion_marks,
);
