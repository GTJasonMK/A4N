use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, TaskKind, character_ai_actions, character_conflict_actions,
    finish_web_task, i64_arg, optional_i32_field, optional_string_field, register_web_task,
    required_json_i32, sanitize_optional_task_id, string_action_result, string_arg, to_value,
    web_fire_and_forget_progress_reporter, web_progress_reporter,
};

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
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let $project_id = i64_arg($args, "projectId")?;
            $(let $arg: $arg_ty = $parse;)*
            let scope = $scope($state.runtime(), $user_id, $project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                task_id.clone(),
                TaskKind::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            );
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter(task.clone());
            let runtime_progress = character_ai_actions::CharacterAiProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
                task_id.as_deref(),
            );
            let $progress = &runtime_progress;
            let result = $run.await;
            let value = finish_web_task(
                $state,
                task_id,
                string_action_result(
                    result,
                    task.as_ref().is_some_and(|handle| handle.is_cancelled()),
                ),
            )?;
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
        pub(in crate::routes::commands) async fn $name(
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

pub(in crate::routes::commands) async fn discover_character_profiles(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = optional_i32_field(args, "chapterNumber")?;
    let scope = character_ai_actions::character_discovery_task_scope(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
    )
    .await
    .map_err(ApiError::bad_request)?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备角色发现",
    );
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let reporter = web_fire_and_forget_progress_reporter(task.clone());
    let runtime_progress = character_ai_actions::CharacterAiProgress::new(
        Some(&reporter),
        cancel_token.as_ref(),
        task_id.as_deref(),
    );
    let result = character_ai_actions::discover_character_profiles_for_chapters(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
        Some(&runtime_progress),
    )
    .await;
    finish_web_task(
        state,
        task_id,
        string_action_result(
            result,
            task.as_ref().is_some_and(|handle| handle.is_cancelled()),
        ),
    )
}

character_ai_web_preview_command!(
    preview_discover_character_profiles_request,
    |state, user_id, args, project_id|,
    (chapter_number: Option<i32> = optional_i32_field(args, "chapterNumber")?),
    character_ai_actions::preview_discover_character_profiles_request(
        state.runtime(),
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
        state.runtime(),
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
        state.runtime(),
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
        state.runtime(),
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
        state.runtime(),
        user_id,
        project_id,
        &name,
        chapter_number,
    ),
);

pub(in crate::routes::commands) async fn check_character_conflict(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let start_chapter = optional_i32_field(args, "startChapter")?;
    let end_chapter = optional_i32_field(args, "endChapter")?;
    let scope = character_conflict_actions::character_conflict_task_scope(
        state.runtime(),
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
    )
    .await
    .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备角色一致性检查",
    );
    let progress = web_progress_reporter(task);
    let result = character_conflict_actions::check_character_conflict(
        state.runtime(),
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
        &progress,
        task_id.as_deref(),
    )
    .await;
    let rows = finish_web_task(state, task_id, result)?;
    Ok(json!(rows))
}

pub(in crate::routes::commands) async fn preview_check_character_conflict_request(
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
            state.runtime(),
            user_id,
            project_id,
            &name,
            start_chapter,
            end_chapter,
        )
        .await?,
    )
}
