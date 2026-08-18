use serde_json::Value;

use super::super::{
    ApiError, ServerState, TaskKind, character_ai_actions, character_conflict_actions,
    command_input, finish_web_task, register_web_task, reject_unknown_command_args,
    sanitize_task_id, string_action_result, string_arg, to_value,
    web_fire_and_forget_progress_reporter,
};

macro_rules! character_ai_web_task_command {
    (
        $name:ident,
        |$state:ident, $user_id:ident, $args:ident, $input:ident, $progress:ident|,
        $input_ty:ty,
        $scope:path,
        $start_message:expr,
        $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            reject_unknown_command_args(
                $args,
                stringify!($name),
                &["input", "runtime_config_snapshot", "taskId"],
            )?;
            let $input: $input_ty = command_input($args, stringify!($name))?;
            let scope = $scope($state.runtime(), $user_id, &$input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_task_id(string_arg($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                Some(task_id.clone()),
                TaskKind::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )?;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter(task.clone());
            let runtime_progress = character_ai_actions::CharacterAiProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
                Some(task_id.as_str()),
            );
            let $progress = &runtime_progress;
            let result = $run.await;
            let value = finish_web_task(
                $state,
                task.as_ref(),
                string_action_result(
                    result,
                    task.as_ref().is_some_and(|handle| handle.is_cancelled()),
                ),
            )?;
            to_value(value)
        }
    };
}

macro_rules! character_ai_web_preview_command {
    (
        $name:ident,
        |$state:ident, $user_id:ident, $args:ident|,
        ($($arg:ident : $arg_ty:ty = $parse:expr),* $(,)?),
        $preview:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $(let $arg: $arg_ty = $parse;)*
            to_value($preview.await.map_err(ApiError::bad_request)?)
        }
    };
}

character_ai_web_task_command!(
    discover_character_profiles,
    |state, user_id, args, input, progress|,
    character_ai_actions::DiscoverCharacterProfilesInput,
    character_ai_actions::character_discovery_task_scope,
    "正在准备角色发现",
    character_ai_actions::discover_character_profiles(
        state.runtime(),
        user_id,
        input,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_discover_character_profiles_request,
    |state, user_id, args|,
    (input: character_ai_actions::DiscoverCharacterProfilesInput =
        command_input(args, "preview_discover_character_profiles_request")?),
    character_ai_actions::preview_discover_character_profiles_request(
        state.runtime(),
        user_id,
        input,
    ),
);

character_ai_web_task_command!(
    sync_all_character_profiles,
    |state, user_id, args, input, progress|,
    character_ai_actions::SyncAllCharacterProfilesInput,
    character_ai_actions::character_profile_sync_all_task_scope,
    "正在准备批量角色同步",
    character_ai_actions::sync_all_character_profiles_to_chapter(
        state.runtime(),
        user_id,
        input,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_sync_all_character_profiles_request,
    |state, user_id, args|,
    (input: character_ai_actions::SyncAllCharacterProfilesInput =
        command_input(args, "preview_sync_all_character_profiles_request")?),
    character_ai_actions::preview_sync_all_character_profiles_request(
        state.runtime(),
        user_id,
        input,
    ),
);

character_ai_web_task_command!(
    sync_character_from_chapters,
    |state, user_id, args, input, progress|,
    character_ai_actions::SyncCharacterProfileInput,
    character_ai_actions::character_profile_sync_task_scope,
    "正在准备角色同步",
    character_ai_actions::sync_character_profile_from_chapters(
        state.runtime(),
        user_id,
        input,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_sync_character_from_chapters_request,
    |state, user_id, args|,
    (input: character_ai_actions::SyncCharacterProfileInput =
        command_input(args, "preview_sync_character_from_chapters_request")?),
    character_ai_actions::preview_sync_character_from_chapters_request(
        state.runtime(),
        user_id,
        input,
    ),
);

character_ai_web_task_command!(
    check_character_conflict,
    |state, user_id, args, input, progress|,
    character_conflict_actions::CharacterConflictCheckInput,
    character_conflict_actions::character_conflict_task_scope,
    "正在准备角色一致性检查",
    character_conflict_actions::check_character_conflict(
        state.runtime(),
        user_id,
        input,
        Some(progress),
    ),
);

character_ai_web_preview_command!(
    preview_check_character_conflict_request,
    |state, user_id, args|,
    (input: character_conflict_actions::CharacterConflictCheckInput =
        command_input(args, "preview_check_character_conflict_request")?),
    character_conflict_actions::preview_character_conflict_request(
        state.runtime(),
        user_id,
        input,
    ),
);
