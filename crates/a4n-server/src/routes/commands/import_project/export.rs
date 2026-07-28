use super::super::{
    ApiError, command_input, export_files_actions, finish_web_task, i64_arg, optional_string_field,
    register_web_task, sanitize_optional_task_id, string_action_result, string_arg,
    web_fire_and_forget_progress_reporter, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
use serde_json::{Value, json};

macro_rules! export_web_project_read_command {
    (
        $name:ident,
        $scope:path,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $project_id:ident|,
        {$($prepare:tt)*},
        $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let $project_id = i64_arg($args, "projectId")?;
            $($prepare)*
            let scope = $scope($state.runtime(), $user_id, $project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskKind::ExportGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let progress = web_progress_reporter(task);
            let result = async {
                progress(0, scope.total, $progress_message)?;
                let value = $run
                    .await
                    .map_err(a4n_core::error::AppError::InvalidArgument)?;
                progress(scope.total, scope.total, $success_message)?;
                Ok(value)
            }
            .await;
            let value = finish_web_task($state, task_id, result).await?;
            Ok(json!(value))
        }
    };
}

macro_rules! export_web_progress_command {
    (
        $name:ident,
        $scope:expr,
        $start_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $progress:ident|,
        {$($prepare:tt)*},
        $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $($prepare)*
            let scope = $scope.await.map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskKind::ExportGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter(task.clone());
            let $progress =
                export_files_actions::ExportProgress::new(Some(&reporter), cancel_token.as_ref());
            let result = $run.await;
            let value = finish_web_task(
                $state,
                task_id,
                string_action_result(
                    result,
                    task.as_ref().is_some_and(|handle| handle.is_cancelled()),
                ),
            )
            .await?;
            Ok(json!(value))
        }
    };
}

export_web_project_read_command!(
    export_txt_data,
    export_files_actions::export_txt_task_scope,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_txt_data(state.runtime(), user_id, project_id),
);

export_web_project_read_command!(
    export_markdown_data,
    export_files_actions::export_markdown_task_scope,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    |state, user_id, args, project_id|,
    {
        let input: export_files_actions::ExportMarkdownInput =
            command_input(args, "export_markdown_data")?;
    },
    export_files_actions::export_markdown_data(
        state.runtime(),
        user_id,
        project_id,
        &input.title,
        &input.author,
    ),
);

export_web_progress_command!(
    export_epub_data,
    export_files_actions::export_epub_task_scope(state.runtime(), user_id, project_id),
    "正在准备 EPUB 导出",
    "EPUB 导出完成",
    |state, user_id, args, progress|,
    {
        let project_id = i64_arg(args, "projectId")?;
        let title = string_arg(args, "title")?;
        let author = string_arg(args, "author")?;
    },
    export_files_actions::export_epub_data(
        state.runtime(), user_id, project_id, title, author, &progress,
    ),
);

export_web_progress_command!(
    export_pdf_data,
    export_files_actions::export_pdf_task_scope(state.runtime(), user_id, project_id),
    "正在准备 PDF 导出",
    "PDF 导出完成",
    |state, user_id, args, progress|,
    {
        let project_id = i64_arg(args, "projectId")?;
    },
    export_files_actions::export_pdf_data(state.runtime(), user_id, project_id, &progress),
);

export_web_progress_command!(
    export_batch_data,
    export_files_actions::export_batch_task_scope(state.runtime(), user_id, &input),
    "正在准备批量导出",
    "批量导出完成",
    |state, user_id, args, progress|,
    {
        let input: export_files_actions::BatchExportInput =
            command_input(args, "export_batch_data")?;
    },
    export_files_actions::export_batch_data(state.runtime(), user_id, input, &progress,),
);

export_web_project_read_command!(
    export_outline_json_data,
    export_files_actions::export_outline_json_task_scope,
    "正在准备导出大纲 JSON",
    "正在读取大纲数据",
    "大纲 JSON 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_outline_json_data(state.runtime(), user_id, project_id),
);

export_web_project_read_command!(
    export_characters_json_data,
    export_files_actions::export_characters_json_task_scope,
    "正在准备导出角色 JSON",
    "正在读取角色数据",
    "角色 JSON 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_characters_json_data(state.runtime(), user_id, project_id),
);
