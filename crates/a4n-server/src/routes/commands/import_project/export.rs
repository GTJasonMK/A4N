use super::super::{
    ApiError, command_input, export_files_actions, finish_web_task, register_web_task,
    sanitize_task_id, string_action_result, string_arg, web_fire_and_forget_progress_reporter,
    web_progress_reporter,
};
use crate::state::ServerState;
use a4n_api_contract::export::{
    BatchExportInput as ApiBatchExportInput, ExportProjectInput, ExportPublicationInput,
};
use a4n_core::event::channel::TaskKind;
use serde_json::{Value, json};

macro_rules! export_web_read_command {
    (
        $name:ident($input_ty:ty),
        $scope_fn:path,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $scope:ident, $input:ident| $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $input: $input_ty = command_input(args, stringify!($name))?;
            let $scope = $scope_fn($state.runtime(), $user_id, $input.project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let project_id = $scope.project_id;
            let total = $scope.total;
            let task_id = sanitize_task_id(string_arg(args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                Some(task_id.clone()),
                TaskKind::ExportGeneration,
                project_id,
                total,
                $start_message,
            )
            .expect("required export task ID must register a task");
            let progress = web_progress_reporter(Some(task));
            let result = async {
                progress(0, total, $progress_message)?;
                let value = $run
                    .await
                    .map_err(a4n_core::error::AppError::InvalidArgument)?;
                progress(total, total, $success_message)?;
                Ok(value)
            }
            .await;
            let value = finish_web_task($state, Some(task_id), result)?;
            Ok(json!(value))
        }
    };
}

macro_rules! export_web_progress_command {
    (
        $name:ident($input_ty:ty),
        $scope_fn:path,
        $start_message:expr,
        |$state:ident, $user_id:ident, $scope:ident, $input:ident, $progress:ident| $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $input: $input_ty = command_input(args, stringify!($name))?;
            let $scope = $scope_fn($state.runtime(), $user_id, $input.project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let project_id = $scope.project_id;
            let total = $scope.total;
            let task_id = sanitize_task_id(string_arg(args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                Some(task_id.clone()),
                TaskKind::ExportGeneration,
                project_id,
                total,
                $start_message,
            )
            .expect("required export task ID must register a task");
            let cancel_token = task.cancel_token.clone();
            let reporter = web_fire_and_forget_progress_reporter(Some(task.clone()));
            let $progress =
                export_files_actions::ExportProgress::new(Some(&reporter), Some(&cancel_token));
            let result = $run.await;
            let value = finish_web_task(
                $state,
                Some(task_id),
                string_action_result(result, task.is_cancelled()),
            )?;
            Ok(json!(value))
        }
    };
}

export_web_read_command!(
    export_txt_data(ExportProjectInput),
    export_files_actions::export_txt_task_scope,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    |state, user_id, scope, input| export_files_actions::export_txt_data(
        state.runtime(),
        user_id,
        scope.project_id,
    ),
);

export_web_read_command!(
    export_markdown_data(ExportPublicationInput),
    export_files_actions::export_markdown_task_scope,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    |state, user_id, scope, input| export_files_actions::export_markdown_data(
        state.runtime(),
        user_id,
        scope.project_id,
        &input.metadata.title,
        &input.metadata.author,
    ),
);

export_web_progress_command!(
    export_epub_data(ExportPublicationInput),
    export_files_actions::export_epub_task_scope,
    "正在准备 EPUB 导出",
    |state, user_id, scope, input, progress| export_files_actions::export_epub_data(
        state.runtime(),
        user_id,
        scope.project_id,
        input.metadata.title,
        input.metadata.author,
        &progress,
    ),
);

export_web_progress_command!(
    export_pdf_data(ExportProjectInput),
    export_files_actions::export_pdf_task_scope,
    "正在准备 PDF 导出",
    |state, user_id, scope, input, progress| export_files_actions::export_pdf_data(
        state.runtime(),
        user_id,
        scope.project_id,
        &progress,
    ),
);

export_web_progress_command!(
    export_batch_data(ApiBatchExportInput),
    export_files_actions::export_batch_task_scope,
    "正在准备批量导出",
    |state, user_id, scope, input, progress| {
        let (title, author) = input
            .metadata
            .map(|metadata| (Some(metadata.title), Some(metadata.author)))
            .unwrap_or_default();
        export_files_actions::export_batch_data(
            state.runtime(),
            user_id,
            export_files_actions::BatchExportInput {
                project_id: scope.project_id,
                formats: input
                    .formats
                    .into_iter()
                    .map(|format| format.to_string())
                    .collect(),
                title,
                author,
            },
            &progress,
        )
    },
);

export_web_read_command!(
    export_outline_json_data(ExportProjectInput),
    export_files_actions::export_outline_json_task_scope,
    "正在准备导出大纲 JSON",
    "正在读取大纲数据",
    "大纲 JSON 导出完成",
    |state, user_id, scope, input| export_files_actions::export_outline_json_data(
        state.runtime(),
        user_id,
        scope.project_id,
    ),
);

export_web_read_command!(
    export_characters_json_data(ExportProjectInput),
    export_files_actions::export_characters_json_task_scope,
    "正在准备导出角色 JSON",
    "正在读取角色数据",
    "角色 JSON 导出完成",
    |state, user_id, scope, input| export_files_actions::export_characters_json_data(
        state.runtime(),
        user_id,
        scope.project_id,
    ),
);
